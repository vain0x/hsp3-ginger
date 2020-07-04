// trie 型の値として使うクラス

#pragma once

// trie の要素やキーに使える型
//
// JSON と同様。
enum TrieType {
	TRIE_NULL,
	TRIE_TRUE,
	TRIE_FALSE,

	// 64 ビット浮動小数点数
	TRIE_NUMBER,

	// 文字列
	TRIE_STRING,

	// 配列
	//
	// キーは 0 以上 2^31 未満の整数に限られる。
	TRIE_ARRAY,

	// オブジェクト
	//
	// 連想配列のようなもの。キーは文字列に限られる。
	TRIE_OBJECT,

	// 内部専用: メタデータ
	TRIE_META,
};

typedef enum TrieType TrieType;

// trie の1個の値
union TrieValue {
	int int_;
	double double_;
	void* ptr_;
};

typedef union TrieValue TrieValue;

// trie の値を表現する型
//
// 参考: [NaN Boxingというテクニックを知った](http://tanakahx.hatenablog.com/entry/2015/07/29/003757)
struct TrieArena {
	TrieValue const* mem_;
};

typedef struct TrieArena TrieArena;



class Trie {
  private:
	std::map<std::string, std::string> map_;
	std::string_view selected_key_ = "";

	// デバッグ用
	inline static std::size_t s_last_id_;
	std::size_t id_;

  public:
	Trie() : id_(s_last_id_++) {
		auto s = std::string{"trie: ctor "} + std::to_string(id_) +
		         std::string{"\n"};
		OutputDebugStringA(s.data());
	}

	~Trie() {
		auto s = std::string{"trie: dtor "} + std::to_string(id_) +
		         std::string{"\n"};
		OutputDebugStringA(s.data());
	}

	void clear() {
		auto s = std::string{"trie: clear "} + std::to_string(id_) +
		         std::string{"\n"};
		OutputDebugStringA(s.data());

		map_.clear();
	}

	auto selected_key() const -> std::string_view {
		auto s = std::string{"trie: selected_key "} + std::to_string(id_) +
		         std::string{" key="} + std::string{selected_key_} +
		         std::string{"\n"};
		OutputDebugStringA(s.data());

		return selected_key_;
	}

	void select_key(std::string_view key) {
		auto s = std::string{"trie: select_key "} + std::to_string(id_) +
		         std::string{" key="} + std::string{key} + std::string{"\n"};
		OutputDebugStringA(s.data());

		selected_key_ = key;
	}

	void insert(std::string key, std::string value) {
		auto s = std::string{"trie: insert "} + std::to_string(id_) +
		         std::string{" key="} + std::string{key} +
		         std ::string{" => "} + value + std::string{"\n"};
		OutputDebugStringA(s.data());

		map_.insert_or_assign(std::move(key), std::move(value));
	}

	auto find(std::string_view key) const -> std::optional<std::string_view> {
		auto s = std::string{"trie: find "} + std::to_string(id_) +
		         std::string{" key="} + std::string{key} + std::string{"\n"};
		OutputDebugStringA(s.data());

		auto iter = map_.find(std::string{key});
		if (iter == map_.end()) {
			return std::nullopt;
		}

		return std::string_view{iter->second};
	}
};

auto operator==(Trie const& left, Trie const& right) -> bool;

auto operator<(Trie const& left, Trie const& right) -> bool;

inline auto operator!=(Trie const& left, Trie const& right) -> bool {
	return !(left == right);
}

inline auto operator<=(Trie const& left, Trie const& right) -> bool {
	return !(right < left);
}

inline auto operator>(Trie const& left, Trie const& right) -> bool {
	return right < left;
}

inline auto operator>=(Trie const& left, Trie const& right) -> bool {
	return !(left < right);
}
