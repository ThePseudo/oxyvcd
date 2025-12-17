use vcd_reader::*;

#[test]
fn test_simple_vcd() {
    let config = Configuration {
        in_file: "tests/files/simple.vcd",
        separator: ' ',
    };
    let reader = VCDFile::new(config);
    let result: Vec<LineInfo> = reader.into_iter().collect();
    let changes_and_timestamps = result
        .iter()
        .filter(|elem| matches!(elem, LineInfo::Timestamp(_) | LineInfo::Change(_)))
        .count();
    assert_eq!(changes_and_timestamps, 552_299);
    let declarations = result
        .iter()
        .filter(|elem| {
            matches!(
                elem,
                LineInfo::InScope(_) | LineInfo::UpScope | LineInfo::Signal(_)
            )
        })
        .count();
    assert_eq!(declarations, 39751);
}
