#include "pch.h"
#include "vartype_int64.h"

static float ref_fval;						// 返値のための変数
static double dp1;

// コマンドを関数形式で記述したときに呼ばれる。
static auto reffunc(int* type_res, int cmd) -> void* {
	//		関数・システム変数の実行処理 (値の参照時に呼ばれます)
	//
	//			'('で始まるかを調べる
	//
	if (*type != TYPE_MARK) puterror(HSPERR_INVALID_FUNCPARAM);
	if (*val != '(') puterror(HSPERR_INVALID_FUNCPARAM);
	code_next();


	switch (cmd) {							// サブコマンドごとの分岐

	case 0x00:								// float関数

		dp1 = code_getd();					// 整数値を取得(デフォルトなし)
		ref_fval = (float)dp1;				// 返値を設定
		break;

	default:
		puterror(HSPERR_UNSUPPORTED_FUNCTION);
	}

	//			'('で終わるかを調べる
	//
	if (*type != TYPE_MARK) puterror(HSPERR_INVALID_FUNCPARAM);
	if (*val != ')') puterror(HSPERR_INVALID_FUNCPARAM);
	code_next();

	*type_res = vartype_int64_id();		// 返値のタイプを指定する
	return (void*)&ref_fval;
}

// アプリケーション終了時に呼ばれる。
static int termfunc(int option) {
	return 0;
}

int WINAPI DllMain(HINSTANCE hInstance, DWORD fdwReason, PVOID pvReserved) {
	return TRUE;
}

EXPORT void WINAPI hsp3_plugin_init(HSP3TYPEINFO* info) {
	hsp3sdk_init(info);

	info->reffunc = reffunc;
	info->termfunc = termfunc;

	registvar(-1, vartype_int64_init);
}
