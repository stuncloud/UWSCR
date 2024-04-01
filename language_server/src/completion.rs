use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat, InsertTextMode};

pub fn get_snippets() -> Vec<CompletionItem> {
    vec![
        new_snippet(
            "for-in", "forin",
r#"for ${1:item} in ${2:items}
    ${0}
next"#
        ),
        new_snippet(
            "for-to", "forto",
r#"for ${1:i} = ${2} to ${3}
    ${0}
next"#
        ),
        new_snippet(
            "for-to-step", "fortostep",
r#"for ${1:i} = ${2:0} to ${3:0} step ${4:0}
    ${0}
next"#
        ),
    ]
}

fn new_snippet(detail: &str, label: &str, snippet: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        // label_details: Some(CompletionItemLabelDetails { detail: todo!(), description: todo!() }),
        kind: Some(CompletionItemKind::SNIPPET),
        detail: Some(detail.to_string()),
        // documentation: todo!(),
        // deprecated: todo!(),
        // preselect: todo!(),
        // sort_text: todo!(),
        // filter_text: todo!(),
        insert_text: Some(snippet.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        insert_text_mode: Some(InsertTextMode::ADJUST_INDENTATION),
        text_edit: None,
        additional_text_edits: None,
        // command: todo!(),
        // commit_characters: todo!(),
        // data: todo!(),
        // tags: todo!(),
        ..Default::default()
    }
}