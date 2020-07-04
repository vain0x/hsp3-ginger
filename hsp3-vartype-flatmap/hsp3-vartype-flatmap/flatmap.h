#pragma once

#include <cassert>
#include <cstdlib>
#include <map>
#include <optional>
#include <string>
#include <vector>

class FlatMap {
  private:
	std::map<std::string, std::string> map_;
	std::string_view selected_key_ = "";

  public:
	void clear() { map_.clear(); }

	auto selected_key() const -> std::string_view {
		return selected_key_;
	}

	void select_key(std::string_view key) {
		selected_key_ = key;
	}

	void insert(std::string key, std::string value) {
		map_.insert_or_assign(std::move(key), std::move(value));
	}

	auto find(std::string_view key) -> std::optional<std::string_view> {
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
