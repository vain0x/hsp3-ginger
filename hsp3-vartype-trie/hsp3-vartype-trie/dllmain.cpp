// DLL のエントリーポイント

#include "pch.h"

#include "commands.h"
#include "vartype_trie.h"

int WINAPI DllMain(HINSTANCE, DWORD, PVOID) { return TRUE; }

// `#regcmd` に指定する関数。HSP のランタイムから起動時に呼ばれる。
EXPORT void WINAPI hsp3_plugin_init(HSP3TYPEINFO* info) {
	hsp3sdk_init(info);

	registvar(-1, vartype_trie_init);

	commands_init(info);
}
