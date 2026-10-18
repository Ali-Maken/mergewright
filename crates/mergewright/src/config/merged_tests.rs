//! `[graph] merged_branches`: the graph mode setting (ease-pass 06h).

use super::tests::loaded_from_a_file;
use super::*;

fn mode_of(text: &str) -> Result<MergedBranches, String> {
    let loaded = loaded_from_a_file(text);
    MergedBranches::from_config(
        loaded
            .config
            .graph
            .merged_branches
            .as_ref()
            .map(|raw| &raw.0),
    )
}

#[test]
fn unset_folded_and_shown_read_as_their_modes_and_round_trip() {
    assert_eq!(mode_of(""), Ok(MergedBranches::Folded));
    assert_eq!(
        mode_of("[graph]\nmerged_branches = \"folded\"\n"),
        Ok(MergedBranches::Folded)
    );
    assert_eq!(
        mode_of("[graph]\nmerged_branches = \"shown\"\n"),
        Ok(MergedBranches::Shown)
    );
    for mode in [MergedBranches::Folded, MergedBranches::Shown] {
        let raw = toml::Value::String(mode.as_config().to_owned());
        assert_eq!(MergedBranches::from_config(Some(&raw)), Ok(mode));
    }
}

#[test]
fn a_value_it_does_not_take_is_a_problem_in_words_and_the_file_still_loads() {
    for written in ["\"open\"", "true", "3"] {
        let text = format!("[graph]\nmerged_branches = {written}\n");
        assert_eq!(
            mode_of(&text),
            Err(format!(
                "graph.merged_branches = {written} — takes \"folded\" or \"shown\"; using folded"
            )),
        );
    }
}

#[test]
fn shown_raises_no_unknown_key_warning() {
    let loaded = loaded_from_a_file("[graph]\nmerged_branches = \"shown\"\n");
    assert_eq!(loaded.warnings, []);
}
