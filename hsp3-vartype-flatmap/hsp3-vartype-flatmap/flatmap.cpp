// flatmap 型の値の実装

#include "pch.h"

#include "flatmap.h"

auto operator==(FlatMap const&, FlatMap const&) -> bool {
	// FIXME: 実装
	throw HSPERR_UNSUPPORTED_FUNCTION;
}

auto operator<(FlatMap const&, FlatMap const&) -> bool {
	// FIXME: 実装
	throw HSPERR_UNSUPPORTED_FUNCTION;
}
