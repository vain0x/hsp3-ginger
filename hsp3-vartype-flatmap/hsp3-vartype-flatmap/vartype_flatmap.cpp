//　flatmap 型の実装
// 参考: 標準の str 型の実装
//  <https://github.com/onitama/OpenHSP/blob/master/src/hsp3/hspvar_str.cpp>
// 参考: var_assoc.hpi の assoc 型の実装
//  <https://github.com/vain0x/hsp-projects/blob/master/hpi/crouton/src/var_assoc/vt_assoc.cpp>

// PVal::size, PVal::basesize などはランタイム側で使用される？

#include "pch.h"

#include "../hspsdk/hspvar_core.h"
#include "flatmap.h"
#include "hsx_arg_reader.h"
#include "vartype_flatmap.h"

using Element = int;

static char s_type_name[] = "flatmap_int";

// 型ID (`vartype("flatmap")`)
static auto s_flatmap_flag = short{};

// 演算の返り値の型
static auto s_aftertype = (short*)nullptr;

static PVal* s_current_pval;
static int s_current_pval_offset;

// 配列変数の総要素数を数える。
//static auto array_element_count(PVal const* pval) -> int {
//	auto count = 1;
//	for (auto i = 1; i <= 4 && pval->len[i] > 0; i++) {
//		count *= pval->len[i];
//	}
//	return count;
//}

// flatmap 型の変数が常に満たすべき条件を表明する。
inline void assert_flatmap_invariant([[maybe_unused]] PVal const* pval) {
	assert(pval != nullptr);
	assert(pval->flag == s_flatmap_flag);
	assert(pval->pt != nullptr);
	assert(pval->master == nullptr);
	//assert(pval->offset >= 0);
	//assert(pval->offset < pval->len[1]);

	assert(pval->mode == HSPVAR_MODE_MALLOC || pval->mode == HSPVAR_MODE_NONE);
	assert(pval->mode != HSPVAR_MODE_MALLOC || pval->len[1] >= 1);
	assert(pval->size == 0 || pval->size || pval->len[1] * sizeof(Element));
}

//static auto flatmap_element(PVal* pval, std::size_t index) -> Element* {
//	assert_flatmap_invariant(pval);
//	assert((int)index < array_element_count(pval));
//
//	return (Element*)pval->pt + index;
//}

static auto flatmap_varproc_get_size(PDAT const*) -> int {
	return (int)sizeof(Element);
}

// flatmap_t 型の配列変数がいま指している要素のデータへのポインタを取得する。
static auto flatmap_varproc_get_ptr_ptr(PVal* pval) -> PDAT* {
	assert_flatmap_invariant(pval);

	//auto flatmap = flatmap_element(pval, (std::size_t)pval->offset);

	//pval->pt = (char*)flatmap;
	//return (PDAT*)pval->pt;
	return (PDAT*)((Element*)pval->pt + pval->offset);
}

// `varuse`
static auto flatmap_varproc_get_using(PDAT const* pdat) -> int {
	// 直前に ArrayObjectRead によって設定された値を参照する
	PVal* pval = s_current_pval;
	int offset = s_current_pval_offset;

	if (!pval || offset != pval->offset) {
		return 0;
	}

	int len = s_current_pval->len[1];
	if (offset >= len) {
		return 0;
	}

	char* meta = (char*)s_current_pval->pt - len;
	return meta[offset] == 1 ? 1 : 0;
}

// 変数に必要なメモリを確保する。
//
// pval2 != nullptr のときは、pval が保持しているデータを維持しながら、配列を拡張する。
static void flatmap_varproc_alloc(PVal* pval, PVal const* pval2) {
	//assert(pval != nullptr);
	//assert(pval->flag == s_flatmap_flag);

	//// 配列の長さは 1 以上にする。
	//if (pval->len[1] < 1) {
	//	pval->len[1] = 1;
	//}

	//auto new_count = array_element_count(pval);

	//int old_count;
	//int new_size;
	//FlatMap* new_data;
	//if (pval2 != nullptr) {
	//	assert_flatmap_invariant(pval2);

	//	// いまの配列の要素数をバッファサイズから逆算する。
	//	old_count = pval->size / (int)sizeof(FlatMap);

	//	// 要素数が増える場合は指数的に増やす。要素数は減少させない。
	//	if (new_count > old_count) {
	//		new_count *= 2;
	//	} else {
	//		new_count = old_count;
	//	}

	//	new_size = new_count * (int)sizeof(FlatMap);
	//	new_data =
	//	    (FlatMap*)exinfo->HspFunc_expand((char*)pval->master, new_size);
	//} else {
	//	old_count = 0;
	//	new_size = new_count * (int)sizeof(FlatMap);
	//	new_data = (FlatMap*)exinfo->HspFunc_malloc(new_size);
	//}

	//// 新しく確保された要素を初期化する。
	//for (auto i = old_count; i < new_count; i++) {
	//	// placement-new
	//	new (new_data + i) FlatMap{};
	//}

	//pval->flag = s_flatmap_flag;
	//pval->size = new_size;
	//pval->master = new_data;
	//pval->pt = (char*)pval->master;
	//pval->mode = HSPVAR_MODE_MALLOC;

	//assert_flatmap_invariant(pval);

	if (pval->len[1] < 64)
		pval->len[1] = 64;

	// 1要素あたり1バイトの管理領域を割り当てる
	//int size = array_element_count(pval) * sizeof(Element);
	//int old_size = pval2 ? pval2->size : 0;

	char* pt;
	if (pval2 != NULL) {
		//assert(pval2->size % sizeof(Element) == 0);
		//int old_size = pval2->size / sizeof(Element);
		//void* old_buf = (int*)pval2->pt - pval2->size;

		//exinfo->HspFunc_malloc(total_size);
		//exinfo->HspFunc_free(old_buf);

		//pt = exinfo->HspFunc_expand(pval->pt, size);
		throw HSPERR_UNSUPPORTED_FUNCTION;
	} else {
		int len = pval->len[1];
		int total_size = len + len * (sizeof(Element) + sizeof(char*));
		if (total_size % 8 != 0) {
			total_size += 8 - total_size % 8;
		}

		int* buf = (int*)exinfo->HspFunc_malloc(total_size);
		memset(buf, 0, total_size);
		pt = (char*)buf + len;
		{
			char msg[1000];
			sprintf_s(msg, "alloc: %p (len=%d)\n", buf, len);
			OutputDebugStringA(msg);
		}
	}

	// 新規要素を0埋め
	//if (size > old_size) {
	//	memset(pt + old_size, 0, (size - old_size));
	//}

	pval->pt = pt;
	pval->master = nullptr;
	pval->size = pval->len[1] * sizeof(Element);
	pval->mode = HSPVAR_MODE_MALLOC;
}

// 変数が確保したメモリを解放する。
static void flatmap_varproc_free(PVal* pval) {
	if (pval->mode != HSPVAR_MODE_MALLOC) {
		// dup, dupptr はサポートしない
		throw HSPERR_UNSUPPORTED_FUNCTION;
	}

	int len = pval->size / (int)sizeof(Element);
	{
		char* meta = pval->pt - len;
		char** keys = (char**)(pval->pt + len * sizeof(Element));
		for (int i = 0; i < len; i++) {
			if (meta[i] == 1) {
				// occupied
				assert(keys[i] != nullptr);
				exinfo->HspFunc_free(keys[i]);
				keys[i] = nullptr;
			}
		}
	}
	char* buf = (char*)pval->pt - len;
	{
		char msg[1000];
		sprintf_s(msg, "free: %p (len=%d)\n", buf, len);
		OutputDebugStringA(msg);
	}
	exinfo->HspFunc_free(buf);

	pval->mode = HSPVAR_MODE_NONE;
	pval->pt = nullptr;
	pval->master = nullptr;
	pval->size = 0;
}

static void* flatmap_get_block_size(PVal* pval, PDAT* pdat, int* size) {
	*size = pval->size - (int)((char*)pdat - pval->pt);
	return pdat;
}

static void flatmap_varproc_alloc_block(PVal*, PDAT*, int) {
	// pass
}

static auto value_to_str(void const* ptr, short flag) -> char const* {
	return flag <= HSPVAR_FLAG_INT
	           ? (char const*)exinfo->HspFunc_getproc(HSPVAR_FLAG_STR)
	                 ->Cnv(ptr, flag)
	           : (char const*)exinfo->HspFunc_getproc(flag)->CnvCustom(
	                 ptr, HSPVAR_FLAG_STR);
}

// 配列要素の指定 (読み込み時)
// '(' を読んだ直後の状態で呼ばれる。
static auto flatmap_varproc_array_object_read(PVal* pval, int* mptype)
    -> void* {
	assert_flatmap_invariant(pval);

	// キーを受け取る。
	auto status = (GetParamStatus)code_getprm();
	if (!param_status_is_ok(status)) {
		throw HSPERR_INVALID_PARAMETER;
	}

	static auto s_reentrant = false;
	static auto s_key = std::string{};
	if (s_reentrant)
		throw HSPERR_UNSUPPORTED_FUNCTION;

	s_reentrant = true;
	s_key = value_to_str(mpval->pt, mpval->flag);

	// probe
	int len = pval->len[1];
	int found_index = -1;
	{
		char* meta = (char*)pval->pt - len;
		char** keys = (char**)((char*)pval->pt + len * sizeof(Element));
		assert(len >= 1);
		auto h = std::hash<std::string>{}(s_key);
		for (int k = 0; k < 4; k++) {
			int i = (int)((h + (size_t)k) % (size_t)len);
			if (meta[i] == 2) {
				// tombstone
				continue;
			}
			if (meta[i] == 1) {
				// occupied
				assert(keys[i] != nullptr);
				if (s_key != keys[i]) {
					continue;
				}
				found_index = i;
			}
			break;
		}
	}
	s_reentrant = false;

	if (found_index < 0) {
		static int s_invalid = -1;
		*mptype = HSPVAR_FLAG_INT;
		return &s_invalid;
	}

	assert(found_index < len);
	*mptype = HSPVAR_FLAG_INT;
	return &((int*)pval->pt)[found_index];
}

// 配列要素の指定 (書き込み時)
// '(' を読んだ直後の状態で呼ばれる。
static void flatmap_varproc_array_object(PVal* pval) {
	assert_flatmap_invariant(pval);

	auto status = (GetParamStatus)code_getprm();
	if (!param_status_is_ok(status)) {
		throw HSPERR_INVALID_PARAMETER;
	}

	static auto s_key = std::string{};
	s_key = value_to_str(mpval->pt, mpval->flag);

	static auto s_reentrant = false;
	if (s_reentrant)
		throw HSPERR_UNSUPPORTED_FUNCTION;
	s_reentrant = true;

	// probe
	int found_index = -1;
	bool occupied = false;
	{
		int len = pval->len[1];
		char* meta = (char*)pval->pt - len;
		char** keys = (char**)((char*)pval->pt + len * sizeof(Element));
		assert(len >= 1);
		auto h = std::hash<std::string>{}(s_key);
		for (int k = 0; k < 4; k++) {
			int i = (int)((h + (size_t)k) % (size_t)len);
			if (meta[i] == 2) {
				// tombstone
				continue;
			}
			if (meta[i] == 1) {
				// occupied
				assert(keys[i] != nullptr);
				if (s_key != keys[i]) {
					continue;
				}
				found_index = i;
				occupied = true;
				break;
			}
			// vacant
			found_index = i;
			occupied = false;
			break;
		}

		// full
		if (found_index < 0) {
			// TODO: rehash
			throw HSPERR_ARRAY_OVERFLOW;
		}

		int i = found_index;
		if (!occupied) {
			int kn = (int)s_key.length();
			char* key = exinfo->HspFunc_malloc((int)(kn + 1));
			memcpy(key, s_key.data(), s_key.length());
			key[kn] = '\0';

			meta[i] = 1; // occupied
			keys[i] = key;
		}
	}
	s_reentrant = false;

	pval->offset = found_index;
	s_current_pval = pval;
	s_current_pval_offset = found_index;
}

// 配列要素への代入 (HSPVAR_SUPPORT_NOCONVERT 指定時のみ)
static void flatmap_varproc_object_write(PVal* pval, void* data, int flag) {
	assert_flatmap_invariant(pval);

	// value_to_int
	int* pdat = flag == HSPVAR_FLAG_INT
	                ? (int*)data
	                : (flag < HSPVAR_FLAG_USERDEF
	                       ? (int*)exinfo->HspFunc_getproc(flag)->Cnv(
	                             data, HSPVAR_FLAG_INT)
	                       : (int*)exinfo->HspFunc_getproc(HSPVAR_FLAG_INT)
	                             ->CnvCustom(data, flag));

	assert((unsigned)pval->offset < (unsigned)pval->len[1]);
	((int*)pval->pt)[pval->offset] = *(int*)pdat;
	pval->offset = 0;
}

// 代入 (=)
//static void flatmap_varproc_set(PVal* pval, PDAT* pdat, void const* ptr) {
//	// deep copy (すべての要素をコピーした新しいマップを構築する。)
//	**(FlatMap**)pdat = **static_cast<FlatMap const* const*>(ptr);
//}

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
static void flatmap_varproc_eq(PDAT* pdat, void const* ptr) {
	*(int*)pdat = pdat == ptr;
	*s_aftertype = HSPVAR_FLAG_INT;
}

// 比較 (!=)
static void flatmap_varproc_ne(PDAT* pdat, void const* ptr) {
	*(int*)pdat = pdat != ptr;
	*s_aftertype = HSPVAR_FLAG_INT;
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
	p->basesize = sizeof(Element);

	// メモリ確保など
	p->GetSize = flatmap_varproc_get_size;
	p->GetPtr = flatmap_varproc_get_ptr_ptr;
	p->GetUsing = flatmap_varproc_get_using;
	p->Alloc = flatmap_varproc_alloc;
	p->Free = flatmap_varproc_free;
	p->GetBlockSize = flatmap_get_block_size;
	p->AllocBlock = flatmap_varproc_alloc_block;

	// 連想配列
	p->ArrayObjectRead = flatmap_varproc_array_object_read;
	p->ArrayObject = flatmap_varproc_array_object;

	// 代入
	p->ObjectWrite = flatmap_varproc_object_write;
	//p->Set = flatmap_varproc_set;

	// 比較
	p->EqI = flatmap_varproc_eq;
	p->NeI = flatmap_varproc_ne;
	//p->LtI = flatmap_varproc_lt;
	//p->LtEqI = flatmap_less_equal_assign;
	//p->GtI = flatmap_greater_than_assign;
	//p->GtEqI = flatmap_greater_equal_assign;
}
