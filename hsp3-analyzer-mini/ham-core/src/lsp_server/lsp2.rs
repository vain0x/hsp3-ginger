use crate::lang_service::LangService;
use crate::lsp_server::init_config::InitConfig;
use crate::lsp_server::lsp_handler::{server_cap, to_init_config};
use crate::lsp_server::lsp_main::get_options_from_env;
use lsp_server::{Connection, ExtractError, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidOpenTextDocument, Exit, Initialized, Notification as _,
};
use lsp_types::{
    request::GotoDefinition, GotoDefinitionResponse, InitializeParams, ServerCapabilities,
};
use lsp_types::{request::Request as _, OneOf};
use std::error::Error;
use std::path::PathBuf;

pub fn run_server2(hsp3_root: PathBuf) -> Result<(), Box<dyn Error + Sync + Send>> {
    // Note that  we must have our logging only write out to stderr.
    eprintln!("starting generic LSP server");

    let lang_service = LangService::new(hsp3_root, get_options_from_env());

    // Create the transport. Includes the stdio (stdin and stdout) versions but this could
    // also be implemented to use sockets or HTTP.
    let (connection, io_threads) = Connection::stdio();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(
        // TODO: use init_config to compute server_caps
        &server_cap(InitConfig::default()),
        //     ServerCapabilities {
        //     definition_provider: Some(OneOf::Left(true)),
        //     ..Default::default()
        // }
    )
    .unwrap();
    let init_params = match connection.initialize(server_capabilities) {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };
    main_loop(connection, init_params, lang_service)?;
    io_threads.join()?;

    // Shut down gracefully.
    eprintln!("shutting down server");
    Ok(())
}

fn main_loop(
    connection: Connection,
    init_params: serde_json::Value,
    mut model: LangService,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let init_params: InitializeParams = serde_json::from_value(init_params).unwrap();
    model.initialize(init_params.root_uri);
    model.did_initialize();

    eprintln!("starting example main loop");
    for msg in &connection.receiver {
        // eprintln!("got msg: {msg:?}");
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                eprintln!("got request: {req:?}");

                match req.method.as_str() {
                    GotoDefinition::METHOD => {
                        let (id, params) = cast::<GotoDefinition>(req).unwrap();
                        let (doc, pos) = {
                            let p = params.text_document_position_params;
                            (p.text_document, p.position)
                        };

                        let definitions = model.definitions(doc.uri, pos);

                        let result = if definitions.len() == 1 {
                            lsp_types::GotoDefinitionResponse::Scalar(
                                definitions.into_iter().next().unwrap(),
                            )
                        } else {
                            lsp_types::GotoDefinitionResponse::Array(definitions)
                        };

                        connection
                            .sender
                            .send(Message::Response(Response::new_ok(id, result)))?;
                        continue;
                    }
                    _ => {
                        continue;
                    }
                }

                // match cast::<GotoDefinition>(req) {
                //     Ok((id, params)) => {
                //         eprintln!("got gotoDefinition request #{id}: {params:?}");
                //         let result = Some(GotoDefinitionResponse::Array(Vec::new()));
                //         let result = serde_json::to_value(&result).unwrap();
                //         let resp = Response {
                //             id,
                //             result: Some(result),
                //             error: None,
                //         };
                //         connection.sender.send(Message::Response(resp))?;
                //         continue;
                //     }
                //     Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                //     Err(ExtractError::MethodMismatch(req)) => req,
                // };
                // ...
            }
            Message::Response(resp) => {
                eprintln!("got response: {resp:?}");
            }
            Message::Notification(not) => {
                eprintln!("got notification: {not:?}");

                match not.method.as_str() {
                    // Initialized::METHOD => {
                    //     // let params = cast_n::<Initialized>(not).unwrap();
                    //     continue;
                    // }
                    DidOpenTextDocument::METHOD => {
                        let params = cast_n::<DidOpenTextDocument>(not).unwrap();
                        // ls.text_document_did_open(params.text_document);
                        // ls.diagnose();
                        let doc = params.text_document;
                        model.open_doc(doc.uri, doc.version, doc.text);
                        continue;
                    }
                    DidChangeTextDocument::METHOD => {
                        let params = cast_n::<DidChangeTextDocument>(not).unwrap();
                        // ls.text_document_did_open(params.text_document);
                        // ls.diagnose();
                        let text = (params.content_changes.into_iter())
                            .next()
                            .map(|c| c.text)
                            .unwrap_or_default();

                        let doc = params.text_document;
                        model.change_doc(doc.uri, doc.version, text);
                        continue;
                    }
                    Exit::METHOD => {
                        break;
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }
    }
    Ok(())
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}

fn cast_n<N>(not: Notification) -> Result<N::Params, ExtractError<Notification>>
where
    N: lsp_types::notification::Notification,
    N::Params: serde::de::DeserializeOwned,
{
    not.extract(N::METHOD)
}
