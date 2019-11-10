// プリコンパイル済みヘッダーファイル

// ここで #include したヘッダーファイルは、1回のビルドで1回だけコンパイルされます。
// これにより、複数のファイルから使用されるヘッダーファイルが複数回コンパイルされることを防ぎ、
// ビルドのパフォーマンスを向上できます。
// ただし、ここで #include したいずれかのヘッダーファイルが更新されると、すべてが再コンパイルされます。
// 頻繁に更新するファイルをここに追加しないでください。追加すると、パフォーマンス上の利点がなくなります。

#ifndef PCH_H
#define PCH_H

// Windows ヘッダーからほとんど使用されていない部分を除外します。
#define WIN32_LEAN_AND_MEAN

// Windows ヘッダーファイル
#include <windows.h>

// HSPSDK
#include "hspsdk/hsp3struct.h"
#include "hspsdk/hspvar_core.h"
#include "hspsdk/hsp3debug.h"

#endif
