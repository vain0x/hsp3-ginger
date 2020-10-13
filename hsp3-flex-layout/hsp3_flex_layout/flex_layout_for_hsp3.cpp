#include "pch.h"

#include "../../vendor/FlexLayout/src/FlexLayout.h"
#include <cstddef>

#define EXPORT extern "C" __declspec(dllexport)

extern "C" void Flex_initialize();

// API

EXPORT void WINAPI hflex_init() { Flex_initialize(); }

EXPORT FlexNodeRef WINAPI hflex_new_node() { return Flex_newNode(); }

EXPORT void WINAPI hflex_free_node(FlexNodeRef node) { Flex_freeNode(node); }

EXPORT void WINAPI hflex_free_node_recursive(FlexNodeRef node) {
	Flex_freeNodeRecursive(node);
}

EXPORT void WINAPI hflex_insert_child(FlexNodeRef node, FlexNodeRef child,
                                      int index) {
	Flex_insertChild(node, child, (std::size_t)index);
}
EXPORT void WINAPI hflex_add_child(FlexNodeRef node, FlexNodeRef child) {
	Flex_addChild(node, child);
}

EXPORT void WINAPI hflex_remove_child(FlexNodeRef node, FlexNodeRef child) {
	Flex_removeChild(node, child);
}

EXPORT FlexNodeRef WINAPI hflex_get_child(FlexNodeRef node, int index) {
	return Flex_getChild(node, (std::size_t)index);
}

EXPORT int WINAPI hflex_get_children_count(FlexNodeRef node) {
	return Flex_getChildrenCount(node);
}

EXPORT void WINAPI hflex_layout(FlexNodeRef node, double constrained_width,
                                double constrained_height, double scale) {
	Flex_layout(node, (float)constrained_width, (float)constrained_height,
	            (float)scale);
}

EXPORT void WINAPI hflex_print(FlexNodeRef node, int options) {
	Flex_print(node, (FlexPrintOptions)options);
}

// node のプロパティ

EXPORT void WINAPI hflex_set_flex_basis(FlexNodeRef node, double value) {
	Flex_setFlexBasis(node, (float)value);
}

EXPORT void WINAPI hflex_set_width(FlexNodeRef node, double value) {
	Flex_setWidth(node, (float)value);
}

EXPORT void WINAPI hflex_set_height(FlexNodeRef node, double value) {
	Flex_setHeight(node, (float)value);
}

EXPORT void WINAPI hflex_get_result_rect(FlexNodeRef node, double* px,
                                         double* py, double* sx, double* sy) {
	*px = (double)Flex_getResultLeft(node);
	*py = (double)Flex_getResultTop(node);
	*sx = (double)Flex_getResultWidth(node);
	*sy = (double)Flex_getResultHeight(node);
}
