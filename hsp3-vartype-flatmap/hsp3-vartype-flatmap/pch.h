// プリコンパイル済みヘッダー

// ライブラリのヘッダーファイルをまとめて include する。
// このプロジェクトのヘッダーファイルは include しないこと。

// プロジェクトに含まれるすべての *.cpp ファイルの先頭で、このヘッダーファイルを include すること。
// そうしない場合はこのプロパティを設定する:
//     C/C++ > プリコンパイル済みヘッダー > プリコンパイル済みヘッダーを使用しない

#pragma once

// 標準ライブラリ
#define WIN32_LEAN_AND_MEAN
#include <Windows.h>
#include <cassert>
#include <charconv>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <map>
#include <string>

// hspsdk
#include "../hspsdk/hsp3plugin.h"
#include "../hspsdk/hsp3struct.h"
#include "../hspsdk/hspvar_core.h"

// 衝突しやすい名前のマクロを無効化する。
#undef min
#undef max
#undef stat
