#include "pch.h"

#include "flatmap.h"
#include "hsx_arg_reader.h"
#include "vartype_flatmap.h"

// `flatmap` コマンドの番号
constexpr auto CMD_FLATMAP = 0x00;

// 関数やシステム変数の結果を一時的に保存しておくための変数。
static auto s_result_int = int{};
static auto s_result_flatmap = FlatMap{};

// 関数やシステム変数の処理結果
struct CommandResult {
	// 結果の型
	short flag;

	// 結果値へのポインタ
	void* ptr;
};

static auto process_command(int cmd) -> int {
	switch (cmd) {
	case CMD_FLATMAP: {
		// dim と同様

		auto pval = (PVal*)nullptr;
		code_getva(&pval);

		int len[4];
		for (auto i = 0; i < 4; i++) {
			len[i] = code_getdi(0);
		}

		exinfo->HspFunc_dim(pval, vartype_flatmap_flag(), 0, len[0], len[1],
		                    len[2], len[3]);
		break;
	}
	default:
		assert(false);
		throw HSPERR_UNSUPPORTED_FUNCTION;
	}

	return RUNMODE_RUN;
}

static auto var_to_string(short flag, void const* ptr) -> std::string {
	auto s = (char const*)exinfo->HspFunc_getproc(flag)->CnvCustom(ptr, HSPVAR_FLAG_STR);
	return std::string{ s };
}

static auto process_command_as_func(int cmd) -> CommandResult {
	switch (cmd) {
	case CMD_FLATMAP: {
		// flatmap(キー1, 値1, キー2, 値2, ...)

		s_result_flatmap.clear();
		while (true) {
			auto at_end = false;

			// キーを取得する。
			auto key = std::string{};
			{
				auto status = (GetParamStatus)code_getprm();

				switch (status) {
				case GetParamStatus::Ok:
				case GetParamStatus::OkFinal:
					key = var_to_string(mpval->flag, mpval->pt);
					break;

				case GetParamStatus::Default:
					break;

				default:
					at_end = true;
					break;
				}

				if (at_end) {
					break;
				}
			}

			// 値を取得する。
			auto value = std::string{};
			{
				auto status = (GetParamStatus)code_getprm();

				switch (status) {
				case GetParamStatus::Ok:
				case GetParamStatus::OkFinal:
					value = var_to_string(mpval->flag, mpval->pt);
					break;

				case GetParamStatus::Default:
					break;

				default:
					at_end = true;
					break;
				}
			}

			s_result_flatmap.insert(std::move(key), std::move(value));

			if (at_end) {
				break;
			}
		}

		return {vartype_flatmap_flag(), &s_result_flatmap};
	}

	default:
		assert(false);
		throw HSPERR_UNSUPPORTED_FUNCTION;
	}
}

static auto process_command_as_system_var(int cmd) -> CommandResult {
	switch (cmd) {
	case CMD_FLATMAP:
		// 型IDを返す。
		s_result_int = vartype_flatmap_flag();
		return {HSPVAR_FLAG_INT, &s_result_int};

	default:
		assert(false);
		throw HSPERR_UNSUPPORTED_FUNCTION;
	}
}

// コマンドが命令として使用されたときに呼ばれる。
static auto cmdfunc(int cmd) -> int {
	code_next();
	return process_command(cmd);
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
static auto termfunc(int) -> int { return 0; }

void commands_init(HSP3TYPEINFO* info) {
	info->cmdfunc = cmdfunc;
	info->reffunc = reffunc;
	info->termfunc = termfunc;
}
