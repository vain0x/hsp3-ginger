/// 字句の種類
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[must_use]
pub(crate) enum TokenKind {
    /// ファイルの終わり (end of file)
    Eof,
    /// 改行やファイル終端の直前 (end of statement)
    Eos,
    /// 改行でない空白。連続する複数の空白を1個のトークンとする。
    Blank,
    /// 改行文字。
    /// 改行文字は \n (LF) または \r\n (CRLF)。
    /// 改行の直後に空白がある場合、それも含める。連続する複数の「改行と空白の繰り返し」を1個のトークンとする。
    Newlines,
    /// コメント
    Comment,
    /// その他 (不明な文字)
    Bad,
    /// 整数または浮動小数点数のリテラル
    Number,
    /// 文字リテラル
    Char,
    /// 文字列リテラル
    Str,
    /// 識別子 (`@` も含む)
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
            TokenKind::Newlines | TokenKind::Blank | TokenKind::Comment | TokenKind::Bad => true,
            _ => false,
        }
    }

    /// 後続トリビアか？
    /// 先行トリビアと違って、改行は含まない。
    pub(crate) fn is_trailing_trivia(self) -> bool {
        match self {
            TokenKind::Blank | TokenKind::Comment | TokenKind::Bad => true,
            _ => false,
        }
    }

    pub(crate) fn is_space(self) -> bool {
        match self {
            TokenKind::Blank | TokenKind::Newlines => true,
            _ => false,
        }
    }
}
