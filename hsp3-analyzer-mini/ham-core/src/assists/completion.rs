use super::to_loc;
use crate::{
    analysis::{
        analyze::ACompletionItem, comment::calculate_details, integrate::AWorkspaceAnalysis,
        ASymbolKind,
    },
    lang_service::docs::Docs,
};
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList, Documentation, Position, Url};

pub(crate) fn incomplete_completion_list() -> CompletionList {
    CompletionList {
        is_incomplete: true,
        items: vec![],
    }
}

pub(crate) fn completion(
    uri: Url,
    position: Position,
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
    other_items: &[CompletionItem],
) -> Option<CompletionList> {
    let mut items = vec![];

    let loc = to_loc(&uri, position, docs)?;

    for item in wa.collect_completion_items(loc) {
        match item {
            ACompletionItem::Symbol(symbol) => {
                let details = calculate_details(&symbol.comments);

                use CompletionItemKind as K;

                let kind = match symbol.kind {
                    ASymbolKind::Unresolved => K::Text,
                    ASymbolKind::Command | ASymbolKind::CommandOrFunc | ASymbolKind::Func => {
                        K::Function
                    }
                    ASymbolKind::CommandOrFuncOrVar | ASymbolKind::PreProc => K::Keyword,
                    ASymbolKind::Const => K::Constant,
                    ASymbolKind::Directory => K::Folder,
                    ASymbolKind::Enum => K::EnumMember,
                    ASymbolKind::Field => K::Field,
                    ASymbolKind::File => K::File,
                    ASymbolKind::Label => K::Reference, // ?
                    ASymbolKind::Module => K::Module,
                    ASymbolKind::Param => K::Variable, // FIXME: intなどの値のパラメータはconstant、変数などの参照渡しパラメータはvariable
                    ASymbolKind::StaticVar => K::Variable,
                    ASymbolKind::Type => K::Class,
                };

                items.push(CompletionItem {
                    kind: Some(kind),
                    label: symbol.name.to_string(),
                    detail: details.desc.map(|s| s.to_string()),
                    documentation: if details.docs.is_empty() {
                        None
                    } else {
                        Some(Documentation::String(details.docs.join("\r\n\r\n")))
                    },
                    ..CompletionItem::default()
                });
            }
        }
    }

    items.extend(other_items.iter().cloned());

    Some(CompletionList {
        is_incomplete: false,
        items,
    })
}
