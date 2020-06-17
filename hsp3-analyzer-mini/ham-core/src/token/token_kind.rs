/// 字句の種類
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TokenKind {
    /// ファイルの終わり (end of file)
    Eof,
    /// 改行やファイル終端の直前 (end of statement)
    Eos,
    /// 改行 (end of line)
    Eol,
    /// 改行でない空白
    Space,
    /// コメント
    Comment,
    /// その他 (不明な文字)
    Other,
    /// 整数または浮動小数点数のリテラル
    Number,
    /// 文字リテラル
    Char,
    /// 文字列リテラル
    Str,
    /// 識別子
    Ident,
    /// `(`
    LeftParen,
    /// `)`
    RightParen,
    /// `{`
    LeftBrace,
    /// `}`
    RightBrace,
    /// `<`
    LeftAngle,
    /// `>`
    RightAngle,
    /// `&`
    And,
    /// `&&`
    AndAnd,
    /// `&=`
    AndEqual,
    /// `\`
    Backslash,
    /// `\=`
    BackslashEqual,
    ///  !`
    Bang,
    /// `!=`
    BangEqual,
    /// `:`
    Colon,
    /// `,`
    Comma,
    /// `.`
    Dot,
    /// `=`
    Equal,
    /// `==`
    EqualEqual,
    /// `#`
    Hash,
    /// `^`
    Hat,
    /// `^=`
    HatEqual,
    /// `<=`
    LeftEqual,
    /// `<<`
    LeftShift,
    /// `-`
    Minus,
    /// `-=`
    MinusEqual,
    /// `--`
    MinusMinus,
    /// `%`
    Percent,
    /// `|`
    Pipe,
    /// `|=`
    PipeEqual,
    /// `||`
    PipePipe,
    /// `+`
    Plus,
    /// `+=`
    PlusEqual,
    /// `++`
    PlusPlus,
    /// `>=`
    RightEqual,
    /// `>>`
    RightShift,
    /// `/`
    Slash,
    /// `/=`
    SlashEqual,
    /// `->`
    SlimArrow,
    /// `*`
    Star,
    /// `*=`
    StarEqual,
}

impl TokenKind {
    /// 先行トリビアか？
    /// 先行トリビアには改行も含まれる。
    /// (空白やコメントなど、構文上の役割を持たないトークンをトリビアと呼ぶ。)
    pub(crate) fn is_leading_trivia(self) -> bool {
        match self {
            TokenKind::Eol | TokenKind::Space | TokenKind::Comment | TokenKind::Other => true,
            _ => false,
        }
    }

    /// 後続トリビアか？
    /// 先行トリビアと違って、改行は含まない。
    pub(crate) fn is_trailing_trivia(self) -> bool {
        match self {
            TokenKind::Space | TokenKind::Comment | TokenKind::Other => true,
            _ => false,
        }
    }
}
