#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaunchRequestArgs {
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
pub(crate) struct Thread {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct StackFrame {
    pub id: i64,
    pub name: String,
    pub line: usize,
    pub source: Source,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Source {
    pub name: String,
    pub path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Scope {
    pub name: String,
    pub variables_reference: i64,
    pub expensive: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Variable {
    pub name: String,
    pub value: String,
    #[serde(rename = "type")]
    pub ty: Option<String>,
    pub variables_reference: i64,
    pub indexed_variables: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "command", content = "arguments", rename_all = "camelCase")]
pub(crate) enum Request {
    Options {
        args: LaunchRequestArgs,
    },
    SetExceptionBreakpoints {
        filters: Vec<String>,
    },
    ConfigurationDone,
    Threads,
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
pub(crate) enum Response {
    Initialize,
    SetExceptionBreakpoints,
    ConfigurationDone,
    Launch,
    Threads {
        threads: Vec<Thread>,
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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum Msg {
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
pub(crate) enum Event {
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
