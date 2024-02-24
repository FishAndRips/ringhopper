use super::*;

#[test]
fn test_argument_parser_help() {
    let parser_help = CommandLineParser::new("Test", "Test")
        .add_help_with_callback(|_| Err("All good!".to_owned()))
        .parse_strs(&["-hdoesn't matter what is after that because argument parsing stops at --help"])
        .err()
        .unwrap();

    assert_eq!(parser_help, "All good!");

    let parser_help = CommandLineParser::new("Test", "Test")
        .add_help_with_callback(|_| Err("All good!".to_owned()))
        .parse_strs(&["--help", "--more", "--arguments", "--doesn't matter too"])
        .err()
        .unwrap();

    assert_eq!(parser_help, "All good!");
}

#[test]
fn test_argument_custom() {
    let parser_data_set = CommandLineParser::new("Test", "Test")
        .set_required_extra_parameters(1)
        .add_custom_parameter(Parameter::single("test", 'T', "test", "", None))
        .add_custom_parameter(Parameter::single("test-unset", 'U', "test", "", None))
        .add_custom_parameter(Parameter::single("test-take", 'Y', "test", "", Some(CommandLineValueType::Float)))
        .parse_strs(&["--test", "extra", "--test-take", "0.0"]).unwrap();

    assert!(parser_data_set.get_custom("test").is_some());
    assert!(parser_data_set.get_custom("test-unset").is_none());
    assert!(parser_data_set.get_custom("test-take").is_some_and(|t| t[0].float() == 0.0));
    assert_eq!(parser_data_set.get_extra(), &["extra".to_owned()]);

    let parser_data_set2 = CommandLineParser::new("Test", "Test")
        .add_custom_parameter(Parameter::single("test", 'T', "test", "", None))
        .add_custom_parameter(Parameter::single("test-unset", 't', "test", "", None))
        .add_custom_parameter(Parameter::single("test-take-uint", '1', "test", "", Some(CommandLineValueType::UInteger)))
        .add_custom_parameter(Parameter::single("test-take-int", '2', "test", "", Some(CommandLineValueType::Integer)))
        .add_custom_parameter(Parameter::single("test-take-ushort", '3', "test", "", Some(CommandLineValueType::UShort)))
        .add_custom_parameter(Parameter::single("test-take-short", '4', "test", "", Some(CommandLineValueType::Short)))
        .add_custom_parameter(Parameter::single("test-take-string", '5', "test", "", Some(CommandLineValueType::String)))
        .add_custom_parameter(Parameter::single("test-take-path", '6', "test", "", Some(CommandLineValueType::Path)))
        .add_custom_parameter(Parameter::new("test-take-multiple", 'Y', "test", "", Some(CommandLineValueType::Float), 1, None, true, false))
        .parse_strs(&["-TYYY12", "0.0", "1.0", "2.0", "712786123", "-4237964", "-3465", "65535", "-32767", "something.txt", "hello world"]).unwrap();

    assert!(parser_data_set2.get_custom("test").is_some());
    assert!(parser_data_set2.get_custom("test-take-multiple").is_some_and(|t| t[0].float() == 0.0 && t[1].float() == 1.0 && t[2].float() == 2.0));
    assert!(parser_data_set2.get_custom("test-take-uint").is_some_and(|t| t[0].uinteger() == 712786123));
    assert!(parser_data_set2.get_custom("test-take-int").is_some_and(|t| t[0].integer() == -4237964));
    assert!(parser_data_set2.get_custom("test-take-ushort").is_some_and(|t| t[0].ushort() == 65535));
    assert!(parser_data_set2.get_custom("test-take-short").is_some_and(|t| t[0].short() == -32767));
    assert!(parser_data_set2.get_custom("test-take-string").is_some_and(|t| t[0].string() == "hello world"));
    assert!(parser_data_set2.get_custom("test-take-path").is_some_and(|t| t[0].path().to_str().unwrap() == "something.txt"));
}
