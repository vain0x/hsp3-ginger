/// 字句の種類
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TokenKind {
    /// ファイルの終わり (end of file)
    Eof,
    /// 改行やファイル終端の直前 (end of line)
    Eol,
    /// 改行や空白
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
    /// 行頭にある `#`。
    /// 行頭にない `#` は Other になる。
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
