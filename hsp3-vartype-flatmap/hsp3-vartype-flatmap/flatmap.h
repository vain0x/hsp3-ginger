// flatmap 型の値として使うクラス

#pragma once

#include <Windows.h>
#include <cassert>
#include <cstdlib>
#include <map>
#include <optional>
#include <string>
#include <vector>

// FIXME: フラットな実装にする
class FlatMap {
  private:
	std::map<std::string, std::string> map_;
	std::string_view selected_key_ = "";

	// デバッグ用
	inline static std::size_t s_last_id_;
	std::size_t id_;

  public:
	FlatMap() : id_(s_last_id_++) {
		auto s = std::string{"flatmap: ctor "} + std::to_string(id_) +
		         std::string{"\n"};
		OutputDebugStringA(s.data());
	}

	~FlatMap() {
		auto s = std::string{"flatmap: dtor "} + std::to_string(id_) +
		         std::string{"\n"};
		OutputDebugStringA(s.data());
	}

	void clear() {
		auto s = std::string{"flatmap: clear "} + std::to_string(id_) +
		         std::string{"\n"};
		OutputDebugStringA(s.data());

		map_.clear();
	}

	auto selected_key() const -> std::string_view {
		auto s = std::string{"flatmap: selected_key "} + std::to_string(id_) +
		         std::string{" key="} + std::string{selected_key_} +
		         std::string{"\n"};
		OutputDebugStringA(s.data());

		return selected_key_;
	}

	void select_key(std::string_view key) {
		auto s = std::string{"flatmap: select_key "} + std::to_string(id_) +
		         std::string{" key="} + std::string{key} + std::string{"\n"};
		OutputDebugStringA(s.data());

		selected_key_ = key;
	}

	void insert(std::string key, std::string value) {
		auto s = std::string{"flatmap: insert "} + std::to_string(id_) +
		         std::string{" key="} + std::string{key} +
		         std ::string{" => "} + value + std::string{"\n"};
		OutputDebugStringA(s.data());

		map_.insert_or_assign(std::move(key), std::move(value));
	}

	auto find(std::string_view key) const -> std::optional<std::string_view> {
		auto s = std::string{"flatmap: find "} + std::to_string(id_) +
		         std::string{" key="} + std::string{key} + std::string{"\n"};
		OutputDebugStringA(s.data());

		auto iter = map_.find(std::string{key});
		if (iter == map_.end()) {
			return std::nullopt;
		}

		return std::string_view{iter->second};
	}
};

auto operator==(FlatMap const& left, FlatMap const& right) -> bool;

auto operator<(FlatMap const& left, FlatMap const& right) -> bool;

inline auto operator!=(FlatMap const& left, FlatMap const& right) -> bool {
	return !(left == right);
}

inline auto operator<=(FlatMap const& left, FlatMap const& right) -> bool {
	return !(right < left);
}

inline auto operator>(FlatMap const& left, FlatMap const& right) -> bool {
	return right < left;
}

inline auto operator>=(FlatMap const& left, FlatMap const& right) -> bool {
	return !(left < right);
}
