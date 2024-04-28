use std::borrow::Cow;

use primitives::{dynamic::DynamicTagData, primitive::{TagPath, TagReference}, tag::PrimaryTagStructDyn};
use ringhopper_structs::group_supported_on_engine;
use crate::tag::tree::TagTree;
use super::{VerifyContext, VerifyResult};

pub fn verify_dependencies<T: TagTree + Send + Sync>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, context: &VerifyContext<T>, result: &mut VerifyResult) {
    fn iterate_dependencies_recursively<D: DynamicTagData + ?Sized, T: TagTree + Send + Sync>(
        data: &D,
        result: &mut VerifyResult,
        context: &VerifyContext<T>,
        stack: &mut Vec<Cow<str>>
    ) {
        for field_name in data.fields() {
            let make_stack_path = || if stack.is_empty() {
                String::new()
            }
            else {
                let mut stack_path = String::new();
                for i in stack.iter() {
                    stack_path += i;
                    stack_path += ".";
                }
                stack_path
            };

            let field = data.get_field(field_name).unwrap();
            if let Some(TagReference::Set(path)) = field.as_any().downcast_ref::<TagReference>() {
                let group = path.group();
                let metadata = match data.get_metadata_for_field(field_name) {
                    Some(n) => n,
                    None => continue
                };
                let allowed = metadata.allowed_references.expect("should have a list of allowed_references");

                if !allowed.contains(&group) {
                    let stack_path = make_stack_path();

                    let mut allowed_list = allowed.first().map(|g| g.as_str().to_owned()).unwrap_or_default();
                    for i in allowed.iter().skip(1) {
                        allowed_list += ",";
                        allowed_list += i.as_str();
                    }

                    result.errors.push(format!("{stack_path}{field_name} references `{group}`, which is not allowed for this reference (allowed references are: {allowed_list})"));
                    continue
                }

                if !group_supported_on_engine(group, context.engine) {
                    let stack_path = make_stack_path();
                    result.errors.push(format!("{stack_path}{field_name} references `{group}`, which is not allowed for engine `{}`", context.engine.name));
                    continue
                }
            }
            if let Some(arr) = field.as_array() {
                for i in 0..arr.len() {
                    let item = arr.get_at_index(i).unwrap();
                    stack.push(Cow::Owned(format!("{field_name}[{i}]")));
                    iterate_dependencies_recursively(item, result, context, stack);
                    stack.pop();
                }
            }

            stack.push(Cow::Borrowed(field_name));
            iterate_dependencies_recursively(field, result, context, stack);
            stack.pop();
        }
    }

    let mut stack = Vec::with_capacity(16);
    iterate_dependencies_recursively(tag, result, context, &mut stack);
}
