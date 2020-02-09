use super::*;
use std::fmt;

/// トークン (token)。ソースコード上の「単語」や「記号」などの種類のこと。
///
/// トークンは、意味的に分割されない最小の単位として定義している。
/// 例えば文字列リテラル `"hello"` を `"`, `hello`, `"` に分解することはありうるが、
/// `hello` をさらに分解する機会はまずない。そのため、これら3つをそれぞれトークンと定めると都合がいい。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Token {
    /// End of file. ソースコードの終端。
    /// 構文解析時の番兵として働く。
    Eof,
    /// End of line. 改行。
    /// トークン数を減らすために、改行の直後にある改行やスペースもこれに含む。
    Eol,
    /// 改行は含まないスペース、またはエスケープされた改行。
    Space,
    /// コメント。改行は除く。
    Comment,
    /// 解釈できない文字
    Other,
    /// 文の終端。
    /// 字句解析時に、改行やファイルの終端の直前に自動で挿入される。
    /// 構文解析時に、文の終わりを表す。(他の言語のセミコロンの役割。)
    Semi,
    /// 2進数リテラルの "0b"
    ZeroB,
    /// 16進数リテラルの "0x"
    ZeroX,
    /// 10進数の整数リテラル ([0-9]+)
    Digit,
    /// 2進数の整数リテラルの数字部分 ([0-1]*)
    Binary,
    /// 16進数の整数リテラルの数字部分 ([0-9A-Fa-f]*)
    Hex,
    /// 整数部。小数点より上の数値。([0-9]*)
    /// `.0` のように整数部がないときは空になる。
    FloatInt,
    /// 小数点 (".")
    FloatPoint,
    /// 小数部。小数点以下の数値。([0-9]+)
    Fraction,
    /// 指数部の文字。([eE])
    ExpChar,
    /// 指数部の符号。([+-])
    ExpSign,
    /// 指数部の数値。([0-9]+)
    ExpDigit,
    /// 文字リテラルの開始 ("'")
    CharStart,
    /// 文字リテラルの終了 ("'")
    CharEnd,
    /// 文字列リテラルの開始 (`"` または `{"`)
    StrStart,
    /// 文字列リテラルの終了 (`"` または `"}`)
    StrEnd,
    /// 文字・文字列リテラル内部のエスケープされない部分
    StrVerbatim,
    /// 文字・文字列リテラル内部のエスケープ1個
    StrEscape,
    /// キーワードではない識別子
    Ident,
    /// 識別子の直後の `@`
    IdentAtSign,
    /// `@` 直後の識別子
    IdentScope,

    // キーワード
    Break,
    Cnt,
    Continue,
    Else,
    End,
    Foreach,
    Gosub,
    Goto,
    If,
    Loop,
    Refdval,
    Refstr,
    Repeat,
    Return,
    Stat,
    Stop,
    Strsize,

    // 約物
    /// `(`
    LeftParen,
    /// `)`
    RightParen,
    /// `<`
    LeftAngle,
    /// `>`
    RightAngle,
    /// `{`
    LeftBrace,
    /// `}`
    RightBrace,
    /// `&&`
    AndAnd,
    /// `&=`
    AndEqual,
    /// `&`
    And,
    /// `\=`
    BackslashEqual,
    /// `\`
    Backslash,
    /// `!=`
    BangEqual,
    ///  !`
    Bang,
    /// `:`
    Colon,
    /// `,`
    Comma,
    /// `$`
    Dollar,
    /// `.`
    Dot,
    /// `==`
    EqualEqual,
    /// `=`
    Equal,
    /// 行頭にある `#`。
    /// 行頭にない `#` は Other になる。
    Hash,
    /// `^=`
    HatEqual,
    /// `^`
    Hat,
    /// `<<`
    LeftShift,
    /// `<=`
    LeftEqual,
    /// `-=`
    MinusEqual,
    /// `--`
    MinusMinus,
    /// `-`
    Minus,
    /// `%=`
    PercentEqual,
    /// `%`
    Percent,
    /// `|=`
    PipeEqual,
    /// `||`
    PipePipe,
    /// `|`
    Pipe,
    /// `+=`
    PlusEqual,
    /// `++`
    PlusPlus,
    /// `+`
    Plus,
    /// `>=`
    RightEqual,
    /// `>>`
    RightShift,
    /// `/=`
    SlashEqual,
    /// `/`
    Slash,
    /// `->`
    SlimArrow,
    /// `*=`
    StarEqual,
    /// `*`
    Star,
}

/// トークンのデータ。永続構文木のリーフノードになる。
#[derive(Clone)]
pub(crate) struct TokenData {
    token: Token,
    text: String,

    source: TokenSource,

    /// 前にスペースや改行がある？
    pub(crate) leading: bool,

    /// 後ろにスペースや改行がある？
    pub(crate) trailing: bool,
}

impl TokenData {
    pub(crate) fn new(token: Token, text: String, source: TokenSource) -> Self {
        TokenData {
            token,
            text,
            source,
            leading: false,
            trailing: false,
        }
    }

    pub(crate) fn token(&self) -> Token {
        self.token
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn source(&self) -> &TokenSource {
        &self.source
    }

    pub(crate) fn len(&self) -> usize {
        self.text.len()
    }

    pub(crate) fn position(&self) -> Position {
        self.text().into()
    }
}

impl fmt::Debug for TokenData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}({:?})", self.token(), self.text())
    }
}
