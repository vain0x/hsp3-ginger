use super::*;

type Px = ParseContext;

pub(crate) enum StmtKind {
    Assign,
    Command,
    Invoke,
}

impl Token {
    pub(crate) fn is_stmt_first(self) -> bool {
        match self {
            Token::Ident | Token::Hash | Token::Star => true,
            _ => self.is_control_keyword(),
        }
    }

    /// このトークンが文の直後に出現しうるか？
    /// (= stmt の FOLLOW 集合に含まれるか？)
    pub(crate) fn is_stmt_follow(self) -> bool {
        self.at_end_of_stmt()
    }

    pub(crate) fn is_plus_or_minus(self) -> bool {
        match self {
            Token::Minus | Token::Plus => true,
            _ => false,
        }
    }

    /// 複合代入を除く、二項演算子
    pub(crate) fn is_simple_binary_operator(self) -> bool {
        match self {
            Token::AndAnd
            | Token::And
            | Token::Backslash
            | Token::BangEqual
            | Token::Bang
            | Token::EqualEqual
            | Token::Equal
            | Token::Hat
            | Token::LeftShift
            | Token::LeftEqual
            | Token::Minus
            | Token::PipePipe
            | Token::Pipe
            | Token::Plus
            | Token::RightEqual
            | Token::RightShift
            | Token::Slash
            | Token::Star => true,
            _ => false,
        }
    }

    /// 複合代入演算子
    pub(crate) fn is_compound_assignment_operator(self) -> bool {
        match self {
            Token::AndEqual
            | Token::BackslashEqual
            | Token::HatEqual
            | Token::MinusEqual
            | Token::MinusMinus
            | Token::PercentEqual
            | Token::PipeEqual
            | Token::PlusEqual
            | Token::PlusPlus
            | Token::SlashEqual
            | Token::StarEqual => true,
            _ => false,
        }
    }

    pub(crate) fn is_assignment_operator(self) -> bool {
        self.is_simple_binary_operator() || self.is_compound_assignment_operator()
    }

    pub(crate) fn is_command_first(self) -> bool {
        self.is_control_keyword() || self == Token::Ident
    }

    pub(crate) fn at_end_of_stmt(self) -> bool {
        self == Token::Eof
            || self == Token::Semi
            || self == Token::RightBrace
            || self == Token::Colon
    }
}

fn parse_end_of_stmt(p: &mut Px) {
    if !p.at_eof() && !p.next().at_end_of_stmt() {
        p.start_node();

        while !p.at_eof() && !p.next().at_end_of_stmt() {
            p.bump();
        }

        p.end_node(NodeKind::Other);
    }

    if !p.at_eof() {
        assert!(p.next().at_end_of_stmt());
        p.bump();
    }
}

fn parse_assign_stmt(p: &mut Px) {
    assert_eq!(p.next(), Token::Ident);

    p.start_node();
    parse_call_expr(p);

    // エラー回復
    if !p.next().at_end_of_stmt() && !p.next().is_assignment_operator() {
        p.start_node();
        while !p.next().at_end_of_stmt() && !p.next().is_assignment_operator() {
            p.bump();
        }
        p.end_node(NodeKind::Other);
    }

    if p.next().is_assignment_operator() {
        p.bump();
    }

    parse_args(p);

    parse_end_of_stmt(p);
    p.end_node(NodeKind::AssignStmt);
}

fn parse_command_stmt_contents(p: &mut Px) {
    if p.next().is_jump_modifier() {
        p.bump();
    }

    if p.next().is_arg_first() {
        parse_args(p);
    }
}

fn parse_command_stmt(p: &mut Px) {
    assert!(p.next().is_command_first());

    p.start_node();

    if p.next().is_control_keyword() {
        p.start_node();
        p.bump();
        p.end_node(NodeKind::Ident);
    } else {
        parse_name(p);
    }

    parse_command_stmt_contents(p);

    parse_end_of_stmt(p);
    p.end_node(NodeKind::CommandStmt);
}

/// メソッド起動文のパース。(`objects(i)->"method" a, b, c` など)
fn parse_invoke_stmt(p: &mut Px) {
    assert_eq!(p.next(), Token::Ident);

    p.start_node();
    parse_call_expr(p);

    // エラー回復
    if !p.next().at_end_of_stmt() && p.next() != Token::SlimArrow {
        p.start_node();
        while !p.next().at_end_of_stmt() && p.next() != Token::SlimArrow {
            p.bump();
        }
        p.end_node(NodeKind::Other);
    }

    p.eat(Token::SlimArrow);

    parse_args(p);

    parse_end_of_stmt(p);
    p.end_node(NodeKind::InvokeStmt);
}

fn look_ahead_stmt(p: &mut Px) -> StmtKind {
    assert_eq!(p.next(), Token::Ident);

    let second = p.nth(1);

    if second == Token::Minus && p.nth(2).at_end_of_stmt() {
        return StmtKind::Assign;
    }

    if second == Token::Minus || second == Token::Star {
        // 曖昧な文。notes.md を参照。
        return StmtKind::Command;
    }

    // mes "hello" のように識別子の直後に原子式があるケースは、代入文ではない。
    // また `on goto ...` のように jump modifier があるケースは命令文に確定。
    if (second != Token::LeftParen && second.is_atom_expr_first()) || second.is_jump_modifier() {
        return StmtKind::Command;
    }

    // `a / ...` のように二項演算子が続く場合は代入文。
    // `a = ...` もこのケースに該当する。
    // ただし `a - ...` のように演算子が式の先頭になりうる場合は除く。
    if second.is_simple_binary_operator() && !second.is_expr_first() {
        return StmtKind::Assign;
    }

    // 添字の後ろを見て判断する。
    let mut i = 1;

    // カッコの深さ。
    let mut paren = 0_usize;

    loop {
        let token = p.nth(i);
        i += 1;

        match token {
            Token::LeftParen => {
                paren += 1;
                continue;
            }
            Token::RightParen => {
                if paren >= 1 {
                    paren -= 1;
                }
                continue;
            }
            Token::Equal if paren == 0 => {
                return StmtKind::Assign;
            }
            Token::SlimArrow => {
                // 矢印が含まれているものは常に起動文とみなす。
                return StmtKind::Invoke;
            }
            token if token.is_compound_assignment_operator() => {
                // 複合代入演算子が含まれているものは常に代入文とみなす。
                return StmtKind::Assign;
            }
            _ if token.at_end_of_stmt() => {
                // `a+` はインクリメント文
                if paren == 0 && i >= 2 && p.nth(i - 2).is_plus_or_minus() {
                    return StmtKind::Assign;
                }

                return StmtKind::Command;
            }
            _ => continue,
        }
    }
}

fn parse_ambiguous_stmt(p: &mut Px) {
    assert_eq!(p.next(), Token::Ident);

    match look_ahead_stmt(p) {
        StmtKind::Assign => parse_assign_stmt(p),
        StmtKind::Command => parse_command_stmt(p),
        StmtKind::Invoke => parse_invoke_stmt(p),
    }
}

fn parse_label_stmt(p: &mut Px) {
    assert_eq!(p.next(), Token::Star);

    p.start_node();

    parse_label_literal(p);
    parse_end_of_stmt(p);

    p.end_node(NodeKind::LabelStmt);
}

fn parse_stmt(p: &mut Px) {
    match p.next() {
        Token::Ident => parse_ambiguous_stmt(p),
        Token::Star => parse_label_stmt(p),
        Token::Hash => parse_pp_stmt(p),
        _ if p.next().is_control_keyword() => parse_command_stmt(p),
        _ => {
            // assert!(p.next().at_end_of_stmt(), "is_stmt_first/at_end_of_stmt bug");
            parse_end_of_stmt(p);
        }
    }
}

pub(crate) fn parse_root(p: &mut Px) {
    while !p.at_eof() {
        // エラー回復
        if !p.next().is_stmt_first() && !p.next().at_end_of_stmt() {
            p.start_node();

            while !p.at_eof() && !p.next().is_stmt_first() {
                p.bump();
            }

            p.end_node(NodeKind::Other);
            continue;
        }

        parse_stmt(p);
    }
}
