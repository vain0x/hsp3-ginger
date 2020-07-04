//　flatmap 型の実装
// 参考: 標準の str 型の実装
//  <https://github.com/onitama/OpenHSP/blob/master/src/hsp3/hspvar_str.cpp>
// 参考: var_assoc.hpi の assoc 型の実装
//  <https://github.com/vain0x/hsp-projects/blob/master/hpi/crouton/src/var_assoc/vt_assoc.cpp>

// PVal::size, PVal::basesize などはランタイム側で使用される？

#include "pch.h"

#include "flatmap.h"
#include "hsx_arg_reader.h"
#include "vartype_flatmap.h"

static char s_type_name[] = "flatmap";

// 型ID
static auto s_flatmap_flag = short{};

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

// flatmap 型の変数が常に満たすべき条件を表明する。
inline void assert_flatmap_invariant([[maybe_unused]] PVal const* pval) {
	assert(pval != nullptr);
	assert(pval->flag == s_flatmap_flag);
	assert(pval->pt != nullptr);
	assert(pval->master != nullptr);
	assert(pval->offset >= 0);
	assert(pval->offset < array_element_count(pval));
}

static auto flatmap_element(PVal* pval, std::size_t index) -> FlatMap* {
	assert_flatmap_invariant(pval);
	assert((int)index < array_element_count(pval));

	return (FlatMap*)pval->master + index;
}

static auto flatmap_value_size(PDAT const*) -> int {
	return (int)sizeof(FlatMap*);
}

// flatmap_t 型の配列変数がいま指している要素のデータへのポインタを取得する。
static auto flatmap_element_ptr(PVal* pval) -> PDAT* {
	assert_flatmap_invariant(pval);

	auto flatmap = flatmap_element(pval, (std::size_t)pval->offset);

	pval->pt = (char*)flatmap;
	return (PDAT*)&pval->pt;
}

// 変数に必要なメモリを確保する。
//
// pval2 != nullptr のときは、pval が保持しているデータを維持しながら、配列を拡張する。
static void flatmap_alloc(PVal* pval, PVal const* pval2) {
	assert(pval != nullptr);
	assert(pval->flag == s_flatmap_flag);

	// 配列の長さは 1 以上にする。
	if (pval->len[1] < 1) {
		pval->len[1] = 1;
	}

	auto new_count = array_element_count(pval);

	int old_count;
	int new_size;
	FlatMap* new_data;
	if (pval2 != nullptr) {
		assert_flatmap_invariant(pval2);

		// いまの配列の要素数をバッファサイズから逆算する。
		old_count = pval->size / (int)sizeof(FlatMap);

		// 要素数が増える場合は指数的に増やす。要素数は減少させない。
		if (new_count > old_count) {
			new_count *= 2;
		} else {
			new_count = old_count;
		}

		new_size = new_count * (int)sizeof(FlatMap);
		new_data =
		    (FlatMap*)exinfo->HspFunc_expand((char*)pval->master, new_size);
	} else {
		old_count = 0;
		new_size = new_count * (int)sizeof(FlatMap);
		new_data = (FlatMap*)exinfo->HspFunc_malloc(new_size);
	}

	// 新しく確保された要素を初期化する。
	for (auto i = old_count; i < new_count; i++) {
		// placement-new
		new (new_data + i) FlatMap{};
	}

	pval->flag = s_flatmap_flag;
	pval->size = new_size;
	pval->master = new_data;
	pval->pt = (char*)pval->master;
	pval->mode = HSPVAR_MODE_MALLOC;

	assert_flatmap_invariant(pval);
}

// 変数が確保したメモリを解放する。
static void flatmap_free(PVal* pval) {
	if (pval->mode != HSPVAR_MODE_MALLOC) {
		pval->mode = HSPVAR_MODE_NONE;
		pval->pt = nullptr;
		pval->master = nullptr;
		return;
	}

	auto data = (FlatMap*)pval->master;
	auto count = pval->size / (int)sizeof(FlatMap);

	// 新しく確保された要素を初期化する。
	for (auto i = 0; i < count; i++) {
		// placement-delete
		data[i].~FlatMap();
	}

	pval->mode = HSPVAR_MODE_NONE;
	pval->pt = nullptr;
	pval->master = nullptr;

	exinfo->HspFunc_free((char*)data);
}

static void* flatmap_block_size(PVal* pval, PDAT* pdat, int* size) {
	*size = pval->size - (int)((char*)pdat - pval->pt);
	return pdat;
}

static void flatmap_alloc_block(PVal*, PDAT*, int) {
	// pass
}

static auto var_to_string(short flag, void const* ptr) -> std::string {
	auto s = flag <= HSPVAR_FLAG_INT
	             ? (char const*)exinfo->HspFunc_getproc(HSPVAR_FLAG_STR)
	                   ->Cnv(ptr, flag)
	             : (char const*)exinfo->HspFunc_getproc(flag)->CnvCustom(
	                   ptr, HSPVAR_FLAG_STR);
	return std::string{s};
}

// 配列要素の指定 (読み込み時)
// '(' を読んだ直後の状態で呼ばれる。
static auto flatmap_element_read(PVal* pval, int* mptype) -> void* {
	assert_flatmap_invariant(pval);

	// キーを受け取る。
	auto status = (GetParamStatus)code_getprm();
	if (!param_status_is_ok(status)) {
		throw HSPERR_INVALID_PARAMETER;
	}

	auto key = var_to_string(mpval->flag, mpval->pt);

	auto value_opt = (*(FlatMap*)pval->master).find(key);
	if (!value_opt) {
		static auto const s_empty_string = std::string{""};
		*mptype = HSPVAR_FLAG_STR;
		return (void*)s_empty_string.data();
	}

	*mptype = HSPVAR_FLAG_STR;
	return (void*)value_opt.value().data();
}

// 配列要素の指定 (書き込み時)
// '(' を読んだ直後の状態で呼ばれる。
static void flatmap_element_write(PVal* pval) {
	assert_flatmap_invariant(pval);

	auto status = (GetParamStatus)code_getprm();
	if (!param_status_is_ok(status)) {
		throw HSPERR_INVALID_PARAMETER;
	}

	static auto s_key = std::string{};
	s_key = var_to_string(mpval->flag, mpval->pt);

	flatmap_element(pval, (std::size_t)pval->offset)->select_key(s_key);
}

// 配列要素への代入 (HSPVAR_SUPPORT_NOCONVERT 指定時のみ)
static void flatmap_element_assign(PVal* pval, void* data, int flag) {
	assert_flatmap_invariant(pval);

	auto flatmap = flatmap_element(pval, (std::size_t)pval->offset);
	auto key = flatmap->selected_key();
	auto value = var_to_string((short)flag, data);
	flatmap->insert(std::string{key}, std::move(value));
}

// 代入 (=)
static void flatmap_assign(PVal*, PDAT* pdat, void const* ptr) {
	// deep copy (すべての要素をコピーした新しいマップを構築する。)
	**(FlatMap**)pdat = **static_cast<FlatMap const* const*>(ptr);
}

// 比較
// CompareFn: 比較関数
template <typename CompareFn>
static void flatmap_do_compare_assign(PDAT* pdat, void const* ptr,
                                      CompareFn compare_fn) {
	auto left = (FlatMap const* const*)pdat;
	auto right = static_cast<FlatMap const* const*>(ptr);

	// 比較が成立するなら 1、しないなら 0 を代入する。
	auto ok = compare_fn(**left, **right) ? 1 : 0;

	*(int*)pdat = ok;

	// 計算結果は int 型になる。
	*s_aftertype = HSPVAR_FLAG_INT;
}

// 比較 (==)
static void flatmap_equal_assign(PDAT* pdat, void const* ptr) {
	flatmap_do_compare_assign(pdat, ptr, std::equal_to<FlatMap>{});
}

// 比較 (!=)
static void flatmap_not_equal_assign(PDAT* pdat, void const* ptr) {
	flatmap_do_compare_assign(pdat, ptr, std::not_equal_to<FlatMap>{});
}

// 比較 (<)
static void flatmap_less_than_assign(PDAT* pdat, void const* ptr) {
	flatmap_do_compare_assign(pdat, ptr, std::less<FlatMap>{});
}

// 比較 (<=)
static void flatmap_less_equal_assign(PDAT* pdat, void const* ptr) {
	flatmap_do_compare_assign(pdat, ptr, std::less_equal<FlatMap>{});
}

// 比較 (>)
static void flatmap_greater_than_assign(PDAT* pdat, void const* ptr) {
	flatmap_do_compare_assign(pdat, ptr, std::greater<FlatMap>{});
}

// 比較 (>=)
static void flatmap_greater_equal_assign(PDAT* pdat, void const* ptr) {
	flatmap_do_compare_assign(pdat, ptr, std::greater_equal<FlatMap>{});
}

EXPORT auto vartype_flatmap_flag() -> short { return s_flatmap_flag; }

// プラグインの初期化時に呼ばれる。
EXPORT void vartype_flatmap_init(HspVarProc* p) {
	s_flatmap_flag = p->flag;
	s_aftertype = &p->aftertype;

	p->vartype_name = s_type_name;
	p->version = 0x001;
	p->support = HSPVAR_SUPPORT_STORAGE | HSPVAR_SUPPORT_FLEXARRAY |
	             HSPVAR_SUPPORT_ARRAYOBJ | HSPVAR_SUPPORT_NOCONVERT;
	p->basesize = sizeof(FlatMap);

	// メモリ確保など
	p->GetSize = flatmap_value_size;
	p->GetPtr = flatmap_element_ptr;
	p->Alloc = flatmap_alloc;
	p->Free = flatmap_free;
	p->GetBlockSize = flatmap_block_size;
	p->AllocBlock = flatmap_alloc_block;

	// 連想配列
	p->ArrayObjectRead = flatmap_element_read;
	p->ArrayObject = flatmap_element_write;

	// 代入
	p->ObjectWrite = flatmap_element_assign;
	p->Set = flatmap_assign;

	// 比較
	p->EqI = flatmap_equal_assign;
	p->NeI = flatmap_not_equal_assign;
	p->LtI = flatmap_less_than_assign;
	p->LtEqI = flatmap_less_equal_assign;
	p->GtI = flatmap_greater_than_assign;
	p->GtEqI = flatmap_greater_equal_assign;
}
