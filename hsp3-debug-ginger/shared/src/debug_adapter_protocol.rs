use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequestArgs {
    /** カレントディレクトリ (現在はワークスペースのルートディレクトリが入る) */
    pub cwd: String,
    /** HSP のインストールディレクトリ */
    pub root: String,
    /** 最初に実行するスクリプトのファイルパス。(例: main.hsp) */
    pub program: String,
    /** 詳細なログを出力するなら true (デバッグ用) */
    pub trace: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Thread {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StackFrame {
    pub id: i64,
    pub name: String,
    pub line: usize,
    pub source: Source,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Source {
    pub name: String,
    pub path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub name: String,
    pub variables_reference: i64,
    pub expensive: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    pub value: String,
    #[serde(rename = "type")]
    pub ty: Option<String>,
    pub variables_reference: i64,
    pub indexed_variables: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "command", content = "arguments", rename_all = "camelCase")]
pub enum Request {
    SetExceptionBreakpoints {
        filters: Vec<String>,
    },
    ConfigurationDone,
    Launch(LaunchRequestArgs),
    Threads,
    Source {
        source: Option<Source>,
    },
    #[serde(rename_all = "camelCase")]
    StackTrace {
        thread_id: i64,
    },
    #[serde(rename_all = "camelCase")]
    Scopes {
        frame_id: i64,
    },
    #[serde(rename_all = "camelCase")]
    Variables {
        variables_reference: i64,
    },
    #[serde(rename_all = "camelCase")]
    Pause {
        thread_id: i64,
    },
    #[serde(rename_all = "camelCase")]
    Continue {
        thread_id: i64,
    },
    #[serde(rename_all = "camelCase")]
    Next {
        thread_id: i64,
    },
    #[serde(rename_all = "camelCase")]
    StepIn {
        thread_id: i64,
    },
    #[serde(rename_all = "camelCase")]
    StepOut {
        thread_id: i64,
    },
    Disconnect {
        restart: bool,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "command", content = "body", rename_all = "camelCase")]
pub enum Response {
    Initialize,
    SetExceptionBreakpoints,
    ConfigurationDone,
    Launch,
    Threads {
        threads: Vec<Thread>,
    },
    Source {
        content: String,
    },
    #[serde(rename_all = "camelCase")]
    StackTrace {
        stack_frames: Vec<StackFrame>,
    },
    #[serde(rename_all = "camelCase")]
    Scopes {
        scopes: Vec<Scope>,
    },
    #[serde(rename_all = "camelCase")]
    Variables {
        variables: Vec<Variable>,
    },
    #[serde(rename_all = "camelCase")]
    Pause {
        thread_id: i64,
    },
    Continue,
    Next,
    StepIn,
    StepOut,
    Disconnect {
        restart: bool,
    },
    /// `success: false` のとき
    Error(Message),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Msg {
    Request {
        seq: i64,
        #[serde(flatten)]
        e: Request,
    },
    Response {
        request_seq: i64,
        success: bool,
        #[serde(flatten)]
        e: Response,
    },
    Event {
        #[serde(flatten)]
        e: Event,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "event", content = "body", rename_all = "camelCase")]
pub enum Event {
    Initialized,
    #[serde(rename_all = "camelCase")]
    Stopped {
        reason: String,
        thread_id: i64,
    },
    #[serde(rename_all = "camelCase")]
    Continued {
        all_threads_continued: bool,
    },
    Terminated {
        restart: bool,
    },
}

// https://microsoft.github.io/debug-adapter-protocol/specification#Types_Message
/// エラーメッセージ
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// エラーの種類を表す一意なID
    id: i64,
    /// メッセージ。変数を `{name}` のかたちで埋め込める
    format: String,
    /// `format` に埋め込まれた変数の値
    variables: Option<HashMap<String, String>>,
    /// エラーをユーザーに見せるか
    show_user: Option<bool>,
}

impl Message {
    pub fn with_message(id: i64, message: String) -> Self {
        let mut map = HashMap::new();
        map.insert("msg".to_string(), message);
        Self {
            id,
            format: "{msg}".into(),
            variables: Some(map),
            show_user: None,
        }
    }
}
