#include "pch.h"
#include "vartype_int64.h"

static char s_type_name[] = "int64";

// int64 の型ID (vartype("int64"))
static auto s_type_id = int{};

// 型変換の結果を一時的に保存しておくための変数
static char s_string_result[100];
static auto s_double_result = double{};
static auto s_int_result = int{};
static auto s_int64_result = std::int64_t{};

// 計算の結果の型
static auto s_aftertype = (short*)nullptr;

// 配列変数の総要素数を数える。
static auto array_element_count(PVal const* pval) -> int {
	auto count = 1;
	for (auto i = 1; i <= 4 && pval->len[i] > 0; i++) {
		count *= pval->len[i];
	}
	return count;
}

static int int64_value_size(PDAT const* pval) {
	return sizeof(std::int64_t);
}

// int64_t 型の配列変数がいま指している要素のデータへのポインタを取得する。
static auto int64_element_ptr(PVal* pval) -> PDAT* {
	return (PDAT*)((std::int64_t*)pval->pt + pval->offset);
}

// 変数に必要なメモリを確保する。
//
// pval2 != nullptr のときは、pval が保持しているデータを維持しながら、配列を拡張する。
static void int64_alloc(PVal* pval, PVal const* pval2) {
	assert(pval != nullptr);
	assert(pval->flag == s_type_id);

	// 配列の長さは 1 以上にする。
	if (pval->len[1] < 1) {
		pval->len[1] = 1;
	}

	auto size = array_element_count(pval) * (int)sizeof(std::int64_t);

	char* new_data;
	if (pval2 != nullptr) {
		if (size > 64 && size > pval->size) {
			size += size / 8;
		} else {
			size = pval->size;
		}

		new_data = exinfo->HspFunc_expand(pval->pt, size);
	} else {
		new_data = exinfo->HspFunc_malloc(size);
		std::memset(new_data, 0, (std::size_t)size);
	}

	pval->flag = (short)s_type_id;
	pval->pt = new_data;
	pval->size = size;
	pval->mode = HSPVAR_MODE_MALLOC;
}

// 変数が確保したメモリを解放する。
static void int64_free(PVal* pval) {
	auto pt = pval->pt;

	pval->mode = HSPVAR_MODE_NONE;
	pval->pt = nullptr;

	exinfo->HspFunc_free(pt);
}

static void* int64_block_size(PVal* pval, PDAT* pdat, int* size) {
	*size = pval->size - (int)((char*)pdat - pval->pt);
	return pdat;
}

static void int64_alloc_block(PVal* pval, PDAT* pdat, int size) {
	// pass
}

// 代入 (=)
static void int64_assign(PVal* pval, PDAT* pdat, void const* ptr) {
	assert(pval != nullptr);
	assert(pval->flag == (short)s_type_id);
	assert(pdat != nullptr);

	*(std::int64_t*)pdat = *static_cast<std::int64_t const*>(ptr);
}

// 加算 (+=)
static void int64_add_assign(PDAT* pdat, void const* ptr) {
	*(std::int64_t*)pdat += *static_cast<std::int64_t const*>(ptr);
	*s_aftertype = (short)s_type_id;
}

// 減算 (-=)
static void int64_sub_assign(PDAT* pdat, void const* ptr) {
	*(std::int64_t*)pdat -= *static_cast<std::int64_t const*>(ptr);
	*s_aftertype = (short)s_type_id;
}

// 乗算 (*=)
static void int64_mul_assign(PDAT* pdat, void const* ptr) {
	*(std::int64_t*)pdat *= *static_cast<std::int64_t const*>(ptr);
	*s_aftertype = (short)s_type_id;
}

// 除算 (/=)
static void int64_div_assign(PDAT* pdat, void const* ptr) {
	auto right = *static_cast<std::int64_t const*>(ptr);
	if (right == 0) {
		throw HSPVAR_ERROR_DIVZERO;
	}

	*(std::int64_t*)pdat /= right;
	*s_aftertype = (short)s_type_id;
}

// 剰余 (\=)
static void int64_mod_assign(PDAT* pdat, void const* ptr) {
	auto right = *static_cast<std::int64_t const*>(ptr);
	if (right == 0) {
		throw HSPVAR_ERROR_DIVZERO;
	}

	*(std::int64_t*)pdat %= right;
	*s_aftertype = (short)s_type_id;
}

// ビット AND (&)
static void int64_bit_and_assign(PDAT* pdat, void const* ptr) {
	*(std::int64_t*)pdat &= *static_cast<std::int64_t const*>(ptr);
	*s_aftertype = (short)s_type_id;
}

// ビット OR (|)
static void int64_bit_or_assign(PDAT* pdat, void const* ptr) {
	*(std::int64_t*)pdat |= *static_cast<std::int64_t const*>(ptr);
	*s_aftertype = (short)s_type_id;
}

// ビット XOR (^)
static void int64_bit_xor_assign(PDAT* pdat, void const* ptr) {
	*(std::int64_t*)pdat ^= *static_cast<std::int64_t const*>(ptr);
	*s_aftertype = (short)s_type_id;
}

// 左シフト (<<)
static void int64_left_shift_assign(PDAT* pdat, void const* ptr) {
	*(std::int64_t*)pdat <<= *static_cast<std::int64_t const*>(ptr);
	*s_aftertype = (short)s_type_id;
}

// 右シフト (>>)
static void int64_right_shift_assign(PDAT* pdat, void const* ptr) {
	*(std::int64_t*)pdat >>= *static_cast<std::int64_t const*>(ptr);
	*s_aftertype = (short)s_type_id;
}

// 比較
// CompareFn: 比較関数
template<typename CompareFn>
static void int64_do_compare_assign(PDAT* pdat, void const* ptr, CompareFn compare_fn) {
	auto left = *(std::int64_t const*)pdat;
	auto right = *static_cast<std::int64_t const*>(ptr);

	// 比較が成立するなら 1、しないなら 0 を代入する。
	*(int*)pdat = compare_fn(left, right) ? 1 : 0;

	// 計算結果は int 型になる。
	*s_aftertype = HSPVAR_FLAG_INT;
}

// 比較 (==)
static void int64_equal_assign(PDAT* pdat, void const* ptr) {
	int64_do_compare_assign(pdat, ptr, std::equal_to<std::int64_t>{});
}

// 比較 (!=)
static void int64_not_equal_assign(PDAT* pdat, void const* ptr) {
	int64_do_compare_assign(pdat, ptr, std::not_equal_to<std::int64_t>{});
}

// 比較 (<)
static void int64_less_than_assign(PDAT* pdat, void const* ptr) {
	int64_do_compare_assign(pdat, ptr, std::less<std::int64_t>{});
}

// 比較 (<=)
static void int64_less_equal_assign(PDAT* pdat, void const* ptr) {
	int64_do_compare_assign(pdat, ptr, std::less_equal<std::int64_t>{});
}

// 比較 (>)
static void int64_greater_than_assign(PDAT* pdat, void const* ptr) {
	int64_do_compare_assign(pdat, ptr, std::greater<std::int64_t>{});
}

// 比較 (>=)
static void int64_greater_equal_assign(PDAT* pdat, void const* ptr) {
	int64_do_compare_assign(pdat, ptr, std::greater_equal<std::int64_t>{});
}

// 値を int64 に型変換する。
// buffer: flag 型の値へのポインタ
// return: int64 型の値へのポインタ (std::int64_t*)
auto int64_convert_from(void const* buffer, int flag) -> void* {
	assert(buffer != nullptr);

	switch (flag) {
	case HSPVAR_FLAG_STR: {
		// FIXME: atoll は範囲外のとき undefined behavior を起こす。strtoll の方がいい？
		s_int64_result = std::atoll(static_cast<char const*>(buffer));
		break;
	}
	case HSPVAR_FLAG_DOUBLE: {
		auto value = *static_cast<double const*>(buffer);
		s_int64_result = (std::int64_t)value;
		break;
	}
	case HSPVAR_FLAG_INT: {
		auto value = *static_cast<int const*>(buffer);
		s_int64_result = (std::int64_t)value;
		break;
	}
	default:
		if (flag == s_type_id) {
			s_int64_result = *static_cast<std::int64_t const*>(buffer);
			break;
		}
		throw HSPVAR_ERROR_TYPEMISS;
	}

	return &s_int64_result;
}

// int64 型の値を flag 型の値に変換する。
// buffer: std::int64_t 型の値へのポインタ
// return: flag 型の値へのポインタ
static auto int64_convert_to(void const* buffer, int flag) -> void* {
	assert(buffer != nullptr);
	auto value = *static_cast<std::int64_t const*>(buffer);

	switch (flag) {
	case HSPVAR_FLAG_STR: {
		auto result = std::to_chars(s_string_result, s_string_result + sizeof(s_string_result), value);
		assert(result.ec == std::errc{});

		*result.ptr = '\0';
		return s_string_result;
	}
	case HSPVAR_FLAG_INT: {
		s_int_result = (int)value;
		return &s_int_result;
	}
	case HSPVAR_FLAG_DOUBLE: {
		s_double_result = (double)value;
		return &s_double_result;
	}
	default:
		if (flag == s_type_id) {
			s_int64_result = value;
			return &s_int64_result;
		}
		throw HSPVAR_ERROR_TYPEMISS;
	}
}

// int64 型の型IDを取得する。
EXPORT auto vartype_int64_id() -> int {
	return s_type_id;
}

// プラグインの初期化時に呼ばれる。
EXPORT void vartype_int64_init(HspVarProc* p) {
	s_type_id = p->flag;
	s_aftertype = &p->aftertype;

	p->vartype_name = s_type_name;
	p->version = 0x100;
	p->support = HSPVAR_SUPPORT_STORAGE | HSPVAR_SUPPORT_FLEXARRAY;
	p->basesize = sizeof(std::int64_t);

	// メモリ確保など
	p->GetSize = int64_value_size;
	p->GetPtr = int64_element_ptr;
	p->Alloc = int64_alloc;
	p->Free = int64_free;
	p->GetBlockSize = int64_block_size;
	p->AllocBlock = int64_alloc_block;

	// 代入
	p->Set = int64_assign;

	// 計算
	p->AddI = int64_add_assign;
	p->SubI = int64_sub_assign;
	p->MulI = int64_mul_assign;
	p->DivI = int64_div_assign;
	p->ModI = int64_mod_assign;
	p->AndI = int64_bit_and_assign;
	p->OrI  = int64_bit_or_assign;
	p->XorI = int64_bit_xor_assign;
	p->LrI = int64_left_shift_assign;
	p->RrI = int64_right_shift_assign;

	// 比較
	p->EqI = int64_equal_assign;
	p->NeI = int64_not_equal_assign;
	p->LtI = int64_less_than_assign;
	p->LtEqI = int64_less_equal_assign;
	p->GtI = int64_greater_than_assign;
	p->GtEqI = int64_greater_equal_assign;

	// 型変換
	p->Cnv = int64_convert_from;
	p->CnvCustom = int64_convert_to;
}
