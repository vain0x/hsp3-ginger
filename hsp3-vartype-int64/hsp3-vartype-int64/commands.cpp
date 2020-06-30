#include "pch.h"
#include "vartype_int64.h"

// `int64` コマンドの番号
constexpr auto CMD_INT64 = 0x00;

// 関数やシステム変数の結果を一時的に保存しておくための変数。
static auto s_result_int = int{};
static auto s_result_int64 = std::int64_t{};

// `code_getprm` の結果
enum class GetParamStatus {
	// 引数を正常に取得した。(次の字句は ')' でない。)
	Ok = PARAM_OK,

	// 引数を正常に取得した。(次の字句は ')' になる。)
	OkFinal = PARAM_SPLIT,

	// 命令の引数リストの末尾 (':' または改行) に達した。引数は取得できなかった。
	EndStmt = PARAM_END,

	// 引数が省略されていて、取得できなかった。
	Default = PARAM_DEFAULT,

	// 関数の引数リストの末尾 (')') に達した。引数は取得できなかった。
	EndFunc = PARAM_ENDSPLIT,
};

// 関数やシステム変数の処理結果
struct CommandResult {
	// 結果の型
	short flag;

	// 結果値へのポインタ
	void* ptr;
};

static auto process_command_as_func(int cmd) -> CommandResult {
	switch (cmd) {
	case CMD_INT64: {
		auto status = (GetParamStatus)code_getprm();
		switch (status) {
		case GetParamStatus::Ok:
		case GetParamStatus::OkFinal: {
			// 受け取った値を int64 型に変換する。
			s_result_int64 = *(std::int64_t const*)int64_convert_from(mpval->pt, mpval->flag);
			break;
		}
		case GetParamStatus::Default:
			s_result_int64 = 0;
			break;

		case GetParamStatus::EndStmt:
		case GetParamStatus::EndFunc:
			throw HSPERR_INVALID_FUNCPARAM;

		default:
			throw HSPERR_UNKNOWN_CODE;
		}

		return { vartype_int64_flag(), &s_result_int64 };
	}
	default:
		assert(false);
		throw HSPERR_UNSUPPORTED_FUNCTION;
	}
}

static auto process_command_as_system_var(int cmd) -> CommandResult {
	switch (cmd) {
	case CMD_INT64:
		// 型IDを返す。
		s_result_int = vartype_int64_flag();
		return { HSPVAR_FLAG_INT, &s_result_int };

	default:
		assert(false);
		throw HSPERR_UNSUPPORTED_FUNCTION;
	}
}

// コマンドが関数やシステム変数として使用されたときに呼ばれる。
static auto reffunc(int* result_flag, int cmd) -> void* {
	if (*type != TYPE_MARK || *val != '(') {
		// システム変数として記述されている。(後ろに '(' がない。)
		auto result = process_command_as_system_var(cmd);
		*result_flag = result.flag;
		return result.ptr;
	}

	// '(' をスキップする。
	code_next();

	auto result = process_command_as_func(cmd);

	// ')' をスキップする。
	if (*type != TYPE_MARK || *val != ')') {
		throw HSPERR_INVALID_FUNCPARAM;
	}
	code_next();

	*result_flag = result.flag;
	return result.ptr;
}

// アプリケーション終了時に呼ばれる。
static int termfunc(int) {
	return 0;
}

void commands_init(HSP3TYPEINFO* info) {
	info->reffunc = reffunc;
	info->termfunc = termfunc;
}
