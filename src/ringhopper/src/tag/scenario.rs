use std::borrow::Cow;
use std::char::REPLACEMENT_CHARACTER;
use definitions::{Scenario, ScenarioScriptNode, ScenarioScriptNodeTable, ScenarioScriptType, ScenarioScriptValueType, ScenarioSourceFile};
use primitives::byteorder::{BigEndian, ByteOrder};
use primitives::dynamic::DynamicEnumImpl;
use primitives::error::{Error, RinghopperResult};
use primitives::parse::SimpleTagData;
use primitives::primitive::{Data, ID, Index, String32};

fn for_each_node_in_scenario<
    From: ByteOrder,
    T: FnMut(&mut [u8], &ScenarioScriptNodeTable),
    U: FnMut(&mut [u8], &ScenarioScriptNode),
    V: FnMut(&mut [u8])
>(
    scenario: &mut Scenario,
    mut on_table: T,
    mut on_set_node: U,
    mut on_garbage_node: V
) -> RinghopperResult<()> {
    let syntax_data = scenario.script_syntax_data.bytes.as_mut_slice();

    let script_node_table = ScenarioScriptNodeTable::read::<From>(syntax_data, 0, syntax_data.len())
        .map_err(|_| Error::InvalidTagData("Can't read script node table from scenario; scripts need recompiled!".to_owned()))?;
    on_table(&mut syntax_data[0..ScenarioScriptNodeTable::simple_size()], &script_node_table);

    let syntax_data_len = syntax_data.len();
    let count = script_node_table.size as usize;
    let max = script_node_table.maximum_count as usize;
    let start = ScenarioScriptNodeTable::simple_size();
    let node_size = ScenarioScriptNode::simple_size();
    let reported_node_size = script_node_table.element_size as usize;

    if node_size != script_node_table.element_size as usize {
        return Err(Error::InvalidTagData(format!("Script node table is corrupt (incorrect node size; expected {node_size}, got {reported_node_size}); scripts need recompiled!")));
    }

    let end = start + count * node_size;
    if end > syntax_data_len {
        return Err(Error::InvalidTagData(format!("Script node table is corrupt (expected at least {end} ({start} + {count} * {node_size}) bytes, but got {syntax_data_len}); scripts need recompiled!")));
    }

    let true_ending = start + max * node_size;
    if true_ending != syntax_data_len {
        return Err(Error::InvalidTagData(format!("Script node table is corrupt (expected exactly {true_ending} ({start} + {max} * {node_size}) bytes, but got {syntax_data_len}); scripts need recompiled!")));
    }

    let nodes = &mut syntax_data[start..true_ending];

    for i in 0..count {
        let offset = i * node_size;
        let node_data = &mut nodes[offset..offset + node_size];
        let node = ScenarioScriptNode::read::<From>(node_data, 0, node_data.len()).unwrap();
        on_set_node(node_data, &node);
    }

    for i in count..max {
        let offset = i * node_size;
        let node_data = &mut nodes[offset..offset + node_size];
        on_garbage_node(node_data);
    }

    Ok(())
}

pub(crate) fn flip_scenario_script_endianness<From: ByteOrder, To: ByteOrder>(scenario: &mut Scenario) -> RinghopperResult<()> {
    for_each_node_in_scenario::<From, _, _, _>(
        scenario,
        |data, table| table.write::<To>(data, 0, data.len()).unwrap(),
        |data, node| node.write::<To>(data, 0, data.len()).unwrap(),
        |data| data.fill(0)
    )
}

pub fn decompile_scripts(scenario: &mut Scenario, scenario_name: &str) -> RinghopperResult<()> {
    check_for_duplicate_scripts(scenario)?;

    if scenario_name.len() > 31 {
        return Err(Error::InvalidTagData(format!("Scenario name is too long to decompile scripts (`{scenario_name}` is {} chars, which is more than 31)", scenario_name.len())))
    }

    let mut all_nodes = Vec::with_capacity(65536);
    for_each_node_in_scenario::<BigEndian, _, _, _>(
        scenario,
        |_, _| (),
        |_, node| all_nodes.push(*node),
        |_| ()
    )?;
    check_scripts_are_ok(scenario, &all_nodes)?;

    let mut tokens = Vec::new();

    for i in &scenario.globals {
        tokens.push(Cow::Borrowed("("));
        tokens.push(Cow::Borrowed("global"));
        tokens.push(Cow::Borrowed(i._type.to_str()));
        tokens.push(Cow::Borrowed(i.name.as_str()));
        decompile_script_block_to_tokens(scenario, &all_nodes, i.initialization_expression_index, &mut tokens, false)?;
        tokens.push(Cow::Borrowed(")"));
    }

    for i in &scenario.scripts {
        tokens.push(Cow::Borrowed("("));
        tokens.push(Cow::Borrowed("script"));
        tokens.push(Cow::Borrowed(i.script_type.to_str()));

        match i.script_type {
            ScenarioScriptType::Static | ScenarioScriptType::Stub => tokens.push(Cow::Borrowed(i.return_type.to_str())),
            _ => ()
        };

        if !i.parameters.items.is_empty() {
            tokens.push(Cow::Borrowed("("));
            tokens.push(Cow::Borrowed(i.name.as_str()));
            for p in &i.parameters {
                tokens.push(Cow::Borrowed("("));
                tokens.push(Cow::Borrowed(p.return_type.to_str()));
                tokens.push(sanitize(Cow::Borrowed(p.name.as_str())));
                tokens.push(Cow::Borrowed(")"));
            }
            tokens.push(Cow::Borrowed(")"));
        }
        else {
            tokens.push(Cow::Borrowed(i.name.as_str()));
        }

        decompile_script_block_to_tokens(scenario, &all_nodes, i.root_expression_index, &mut tokens, true)?;
        tokens.push(Cow::Borrowed(")"));
    }

    if tokens.is_empty() {
        return Ok(())
    }

    let result = format_tokens(&tokens);
    let page_count = result.len();

    if page_count == 0 {
        return Ok(())
    }

    scenario.source_files.items.clear();
    scenario.source_files.items.reserve_exact(page_count);

    // If the scenario name is extraordinarily long, split it
    // 31 (max length of string32) - (hyphen (1) + digit count (log10(page_count) + 1))
    let max_scenario_name_length = (31 - (1 + page_count.ilog10() + 1)) as usize;
    let mut scenario_name = scenario_name;
    if page_count > 1 && scenario_name.len() > max_scenario_name_length {
        let mut split_at = 0;
        for i in scenario_name.char_indices() {
            if i.0 > 16 {
                split_at = i.0;
                break;
            }
        }
        scenario_name = scenario_name.split_at(split_at).0;
    }

    for i in 0..page_count {
        let num = i + 1;
        let name = if page_count == 1 {
            String32::from_str(scenario_name)
        }
        else {
            String32::from_str(&format!("{scenario_name}-{num}"))
        }.unwrap();

        let page_data = &result[i];
        const NAME: &str = env!("CARGO_PKG_NAME");
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        let result = format!(";*\n================================================================================\n\n    {name}.hsc, file {num} of {page_count}\n\n    generated by {NAME} version {VERSION}\n\n================================================================================\n*;\n\n{page_data}");
        scenario.source_files.items.push(ScenarioSourceFile {
            name,
            source: Data::new(result.into_bytes())
        })
    }

    Ok(())
}

const SCRIPT_SPLIT: usize = 512 * 1024;
const RESERVED_CAPACITY: usize = SCRIPT_SPLIT + SCRIPT_SPLIT / 2;

fn format_tokens(tokens: &[Cow<str>]) -> Vec<String> {
    let mut files = Vec::new();
    let mut result;

    macro_rules! next_page {
        () => {
            files.push(String::with_capacity(RESERVED_CAPACITY));
            result = files.last_mut().unwrap();
        };
    }

    next_page!();

    let mut depth = 0usize;
    let mut token_index = 0usize;
    let token_count = tokens.len();

    while token_index < token_count {
        if depth == 0 && result.len() >= SCRIPT_SPLIT {
            next_page!();
        }

        macro_rules! make_spacing {
            () => {
                for _ in 0..depth {
                    *result += "    ";
                }
            }
        }

        if !result.is_empty() && depth == 0 {
            *result += "\n";
        }

        // Put this token down now
        let token = &tokens[token_index];
        token_index += 1;

        if token == "(" {
            make_spacing!();

            // lookahead for the ")"; see if this can be a one-liner
            let mut end_of_one_liner = {
                let mut depth_inner = 1usize;
                let mut end = None;

                for q in token_index..(token_index + 24).min(token_count) {
                    if tokens[q] == "(" {
                        depth_inner += 1;
                    }
                    if tokens[q] == ")" {
                        depth_inner -= 1;
                        if depth_inner == 0 {
                            end = Some(q);
                            break
                        }
                    }
                }

                end
            };

            if end_of_one_liner.is_none() {
                depth += 1;
                let next_token = &tokens[token_index];
                match next_token {
                    n if depth == 1 && n == "script" => {
                        let script_type = &tokens[token_index + 1];
                        let mut script_name_index = if script_type == "static" || script_type == "stub" {
                            token_index + 3
                        }
                        else {
                            token_index + 2
                        };

                        if tokens[script_name_index] == "(" {
                            script_name_index -= 1;
                        }
                        end_of_one_liner = Some(script_name_index);
                    },
                    n if depth == 1 && n == "global" => end_of_one_liner = Some(token_index + 2),
                    n if n == "(" => (),
                    _ => end_of_one_liner = Some(token_index)
                };
            }

            // Put it all in one line if possible
            if let Some(end_of_one_liner) = end_of_one_liner {
                let mut last_token = token;
                *result += token;
                for i in token_index..=end_of_one_liner {
                    let this_token = &tokens[i];
                    if last_token != "(" && this_token != ")" {
                        *result += " ";
                    }
                    last_token = this_token;
                    *result += this_token;
                }
                token_index = end_of_one_liner + 1;
                *result += "\n";
                continue;
            }
            else {
                debug_assert_ne!(depth, usize::MAX);
            }
        }

        else if token == ")" {
            debug_assert_ne!(depth, 0);
            depth -= 1;
            make_spacing!();
        }

        else {
            make_spacing!();
        }

        *result += token;
        *result += "\n";
    }

    files
}

fn decompile_script_block_to_tokens<'a>(
    scenario: &'a Scenario,
    nodes: &Vec<ScenarioScriptNode>,
    block: ID,
    tokens: &mut Vec<Cow<'a, str>>,
    remove_redundant_begin: bool
) -> RinghopperResult<()> {
    let mut next_node = block.index();
    while let Some(n) = next_node.map(|i| &nodes[i as usize]) {
        next_node = n.next_node.index();
        decompile_token(scenario, nodes, n, tokens, remove_redundant_begin)?;
    }
    Ok(())
}

fn decompile_token<'a>(
    scenario: &'a Scenario,
    nodes: &Vec<ScenarioScriptNode>,
    node: &ScenarioScriptNode,
    tokens: &mut Vec<Cow<'a, str>>,
    remove_redundant_begin: bool
) -> RinghopperResult<()> {
    if !node.flags.is_primitive {
        let mut child_id = ID::from(node.data);
        let child_index = child_id
            .index()
            .ok_or_else(|| Error::InvalidTagData("empty function call without even a function name; scripts need recompiled".to_owned()))? as usize;

        let node_peek_node = &nodes[child_index];
        let strip_begin = remove_redundant_begin && get_string_data_for_node(scenario, node_peek_node)? == "begin";

        if strip_begin {
            child_id = node_peek_node.next_node;
        }
        else {
            tokens.push(Cow::Borrowed("("));
        }
        decompile_script_block_to_tokens(scenario, nodes, child_id, tokens, false)?;
        if !strip_begin {
            tokens.push(Cow::Borrowed(")"));
        }
        return Ok(())
    }

    if node.flags.is_global || node.flags.is_local_variable {
        tokens.push(sanitize(get_string_data_for_node(scenario, node)?));
        return Ok(())
    }

    let token = match node._type {
        ScenarioScriptValueType::Boolean => Cow::Borrowed(if bool::from(node.data) { "true" } else { "false" }),
        ScenarioScriptValueType::Real => Cow::Owned(f32::from(node.data).to_string()),
        ScenarioScriptValueType::Short => Cow::Owned(i16::from(node.data).to_string()),
        ScenarioScriptValueType::Long => Cow::Owned(i32::from(node.data).to_string()),
        ScenarioScriptValueType::Void => return Ok(()),
        _ => sanitize(get_string_data_for_node(scenario, node)?)
    };
    tokens.push(token);
    return Ok(())
}

fn sanitize(mut string: Cow<str>) -> Cow<str> {
    if string == "" {
        return Cow::Borrowed("\"\"");
    }
    if string.contains("\"") {
        string = Cow::Owned(string.replace("\"", "\\\""));
    }
    if string.contains("(") || string.contains(")") || string.contains(" ") {
        string = Cow::Owned(format!("\"{string}\""));
    }
    string
}

fn check_for_duplicate_scripts(scenario: &Scenario) -> RinghopperResult<()> {
    for i in 0..scenario.scripts.items.len() {
        let name = scenario.scripts.items[i].name;
        for j in 0..i {
            if name == scenario.scripts.items[j].name {
                return Err(Error::InvalidTagData(format!("Duplicate script {name} baked into the scenario; scripts need recompiled!")))
            }
        }
    }
    for i in 0..scenario.globals.items.len() {
        let name = scenario.globals.items[i].name;
        for j in 0..i {
            if name == scenario.globals.items[j].name {
                return Err(Error::InvalidTagData(format!("Duplicate global {name} baked into the scenario; scripts need recompiled!")))
            }
        }
    }
    Ok(())
}

fn check_scripts_are_ok(scenario: &Scenario, extracted_nodes: &Vec<ScenarioScriptNode>) -> RinghopperResult<()> {
    check_for_duplicate_scripts(scenario)?;

    let mut todo = Vec::with_capacity(64);

    for i in &scenario.scripts {
        check_bad_scripts(i.root_expression_index, extracted_nodes, &mut todo)
            .map_err(|e| Error::InvalidTagData(format!("Script {} has errors: {e}", i.name)))?;
    }

    for i in &scenario.globals {
        check_bad_scripts(i.initialization_expression_index, extracted_nodes, &mut todo)
            .map_err(|e| Error::InvalidTagData(format!("Global {} has errors: {e}", i.name)))?;
    }

    Ok(())
}

fn get_string_data_for_node<'a>(scenario: &'a Scenario, node: &ScenarioScriptNode) -> RinghopperResult<Cow<'a, str>> {
    let offset = node.string_offset as usize;
    let data = &scenario.script_string_data.bytes;

    if offset >= data.len() {
        return Err(Error::InvalidTagData(format!("String data offset 0x{offset:04X} out-of-bounds!")));
    }

    match std::ffi::CStr::from_bytes_until_nul(&data[offset..]) {
        Ok(n) => {
            let string = n.to_string_lossy();
            if let Cow::Owned(q) = string {
                Ok(Cow::Owned(q.replace(REPLACEMENT_CHARACTER, "?")))
            }
            else {
                Ok(string)
            }
        },
        Err(_) => Err(Error::InvalidTagData(format!("String data offset 0x{offset:04X} not null-terminated!")))
    }
}

fn check_bad_scripts(node: ID, extracted_nodes: &Vec<ScenarioScriptNode>, todo: &mut Vec<(Index, usize)>) -> RinghopperResult<()> {
    todo.clear();

    let node_count = extracted_nodes.len();
    let iteration_limit = node_count + 32;
    todo.push((node.index(), 0));

    while let Some((index, depth)) = todo.pop() {
        if depth > iteration_limit {
            return Err(Error::InvalidTagData(format!("Depth exceeds actual node count ({node_count}); infinite recursion detected!")))
        }

        let index = match index {
            Some(n) => n as usize,
            None => return Err(Error::InvalidTagData("Null index referenced!".to_owned()))
        };
        if index >= node_count {
            return Err(Error::InvalidTagData("Out-of-bounds node index referenced!".to_owned()))
        }

        // Loop until we are at the end of the block, adding function calls to the stack
        let mut iterated_nodes = 0usize;
        let mut node = &extracted_nodes[index];
        loop {
            iterated_nodes += 1;
            if iterated_nodes > iteration_limit {
                return Err(Error::InvalidTagData(format!("Block length exceeds actual node count ({node_count}); infinite loop detected!")));
            }

            if !node.flags.is_primitive {
                let id: ID = node.data.into();
                todo.push((id.index(), depth + 1));
            }

            let next_node_index = node.next_node.index();
            let index = match next_node_index {
                Some(n) => n as usize,
                None => break
            };
            if index >= node_count {
                return Err(Error::InvalidTagData("Out-of-bounds node index detected mid-block!".to_owned()));
            }
            node = &extracted_nodes[index];
        }
    }

    Ok(())
}
