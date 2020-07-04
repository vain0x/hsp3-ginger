#pragma once

#include "../hspsdk/hspvar_core.h"

// trie 型の型 ID を取得する。
EXPORT auto vartype_trie_flag() -> short;

EXPORT void vartype_trie_init(HspVarProc*);
