#include "pch.h"

#include "../../vendor/FlexLayout/src/FlexLayout.h"
#include <cassert>
#include <cstddef>

#define EXPORT extern "C" __declspec(dllexport)

extern "C" void Flex_initialize();

// API

static void hflex_check(bool cond) {
	assert(cond && "hflex_check");
	if (!cond) {
		// HSPERR_UNKNOWN_CODE
		throw 1;
	}
}

EXPORT void WINAPI hflex_init() { Flex_initialize(); }

EXPORT FlexNodeRef WINAPI hflex_new_node() { return Flex_newNode(); }

EXPORT void WINAPI hflex_free_node(FlexNodeRef node) {
	if (node != nullptr) {
		Flex_freeNode(node);
	}
}

EXPORT void WINAPI hflex_free_node_recursive(FlexNodeRef node) {
	if (node != nullptr) {
		Flex_freeNodeRecursive(node);
	}
}

EXPORT void WINAPI hflex_insert_child(FlexNodeRef node, FlexNodeRef child,
                                      int index) {
	hflex_check(node != nullptr && child != nullptr);

	Flex_insertChild(node, child, (std::size_t)index);
}

EXPORT void WINAPI hflex_add_child(FlexNodeRef node, FlexNodeRef child) {
	hflex_check(node != nullptr && child != nullptr);

	Flex_addChild(node, child);
}

EXPORT void WINAPI hflex_remove_child(FlexNodeRef node, FlexNodeRef child) {
	hflex_check(node != nullptr && child != nullptr);

	Flex_removeChild(node, child);
}

EXPORT FlexNodeRef WINAPI hflex_get_child(FlexNodeRef node, int index) {
	hflex_check(node != nullptr);

	return Flex_getChild(node, (std::size_t)index);
}

EXPORT int WINAPI hflex_get_children_count(FlexNodeRef node) {
	hflex_check(node != nullptr);

	return Flex_getChildrenCount(node);
}

EXPORT void WINAPI hflex_layout(FlexNodeRef node, double constrained_width,
                                double constrained_height, double scale) {
	hflex_check(node != nullptr);

	Flex_layout(node, (float)constrained_width, (float)constrained_height,
	            (float)scale);
}

EXPORT void WINAPI hflex_print(FlexNodeRef node, int options) {
	hflex_check(node != nullptr);

	Flex_print(node, (FlexPrintOptions)options);
}

// node のプロパティ

EXPORT void WINAPI hflex_set_margin(FlexNodeRef node, double left, double top,
                                    double right, double bottom) {
	hflex_check(node != nullptr);

	Flex_setMarginLeft(node, (float)left);
	Flex_setMarginTop(node, (float)top);
	Flex_setMarginRight(node, (float)right);
	Flex_setMarginBottom(node, (float)bottom);
}

EXPORT void WINAPI hflex_set_flex_flow(FlexNodeRef node, int direction,
                                       int wrap) {
	hflex_check(node != nullptr);

	Flex_setDirection(node, (FlexDirection)direction);
	Flex_setWrap(node, (FlexWrapMode)wrap);
}

EXPORT void WINAPI hflex_set_justify_content(FlexNodeRef node, int align) {
	hflex_check(node != nullptr);

	Flex_setJustifyContent(node, (FlexAlign)align);
}

EXPORT void WINAPI hflex_set_align_items(FlexNodeRef node, int align) {
	hflex_check(node != nullptr);

	Flex_setAlignItems(node, (FlexAlign)align);
}

EXPORT void WINAPI hflex_set_align_content(FlexNodeRef node, int align) {
	hflex_check(node != nullptr);

	Flex_setAlignContent(node, (FlexAlign)align);
}

EXPORT void WINAPI hflex_set_flex(FlexNodeRef node, double grow, double shrink,
                                  double basis) {
	hflex_check(node != nullptr);

	Flex_setFlexGrow(node, (float)grow);
	Flex_setFlexShrink(node, (float)shrink);
	Flex_setFlexBasis(node, (float)basis);
}

EXPORT void WINAPI hflex_set_flex_basis(FlexNodeRef node, double value) {
	hflex_check(node != nullptr);

	Flex_setFlexBasis(node, (float)value);
}

EXPORT void WINAPI hflex_set_width(FlexNodeRef node, double value) {
	hflex_check(node != nullptr);

	Flex_setWidth(node, (float)value);
}

EXPORT void WINAPI hflex_set_height(FlexNodeRef node, double value) {
	hflex_check(node != nullptr);

	Flex_setHeight(node, (float)value);
}

EXPORT void WINAPI hflex_get_result_rect(FlexNodeRef node, double* px,
                                         double* py, double* sx, double* sy) {
	hflex_check(node != nullptr && px != nullptr && py != nullptr &&
	            sx != nullptr && sy != nullptr);

	*px = (double)Flex_getResultLeft(node);
	*py = (double)Flex_getResultTop(node);
	*sx = (double)Flex_getResultWidth(node);
	*sy = (double)Flex_getResultHeight(node);
}
