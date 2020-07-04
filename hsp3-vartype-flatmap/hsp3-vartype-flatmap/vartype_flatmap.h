#pragma once

#include "../hspsdk/hspvar_core.h"

// flatmap 型の型 ID を取得する。
EXPORT auto vartype_flatmap_flag() -> short;

EXPORT void vartype_flatmap_init(HspVarProc* p);
