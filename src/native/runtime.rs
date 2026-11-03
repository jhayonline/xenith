use std::{
    fs,
    path::{Path, PathBuf},
};

/// The native runtime is deliberately C ABI only: generated objects can link
/// with a normal host C compiler without requiring a separately installed VM.
const SOURCE: &str = r#"#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
typedef struct { int8_t pending; unsigned char *message; int64_t message_len; const unsigned char *code; int64_t code_len; const unsigned char *name; int64_t name_len; } XenithError;
static _Thread_local XenithError xenith_error = {0, NULL, 0, NULL, 0, NULL, 0};
static _Thread_local unsigned char xenith_error_slot[256];
static _Thread_local unsigned char xenith_error_message_slot[256];
static _Thread_local unsigned char xenith_error_code_slot[32];
static _Thread_local unsigned char xenith_error_name_slot[64];
void xenith_error_clear(void) { if (xenith_error.message && xenith_error.message != xenith_error_message_slot) free(xenith_error.message); xenith_error = (XenithError){0, NULL, 0, NULL, 0, NULL, 0}; }
int8_t xenith_error_pending(void) { return xenith_error.pending; }
void xenith_error_abort_if_pending(void) { if (xenith_error.pending) { fprintf(stderr, "Xenith error %.*s (%.*s): %.*s\n", (int)xenith_error.code_len, xenith_error.code ? (const char *)xenith_error.code : "unknown", (int)xenith_error.name_len, xenith_error.name ? (const char *)xenith_error.name : "Error", (int)xenith_error.message_len, xenith_error.message ? (const char *)xenith_error.message : "an unknown error occurred"); exit(1); } }
void xenith_error_raise(const unsigned char *message, int64_t message_len, const unsigned char *code, int64_t code_len, const unsigned char *name, int64_t name_len) { xenith_error_clear(); xenith_error.pending = 1; if (message_len > 0 && message_len < (int64_t)sizeof(xenith_error_message_slot)) { xenith_error.message = malloc((size_t)message_len); } if (xenith_error.message) { memcpy(xenith_error.message, message, (size_t)message_len); } else { xenith_error.message = xenith_error_message_slot; message_len = message_len < (int64_t)sizeof(xenith_error_message_slot) ? message_len : (int64_t)sizeof(xenith_error_message_slot) - 1; memcpy(xenith_error.message, message, (size_t)message_len); } xenith_error.message_len = message_len; code_len = code_len < (int64_t)sizeof(xenith_error_code_slot) ? code_len : (int64_t)sizeof(xenith_error_code_slot) - 1; name_len = name_len < (int64_t)sizeof(xenith_error_name_slot) ? name_len : (int64_t)sizeof(xenith_error_name_slot) - 1; memcpy(xenith_error_code_slot, code, (size_t)code_len); memcpy(xenith_error_name_slot, name, (size_t)name_len); xenith_error.code = xenith_error_code_slot; xenith_error.code_len = code_len; xenith_error.name = xenith_error_name_slot; xenith_error.name_len = name_len; }
void xenith_allocation_failure(void) { const unsigned char message[] = "allocation failed"; const unsigned char code[] = "XEN005"; const unsigned char name[] = "Allocation Failure"; xenith_error_raise(message, sizeof(message) - 1, code, sizeof(code) - 1, name, sizeof(name) - 1); }
int64_t xenith_int_divide(int64_t left, int64_t right) { if (right == 0) { const unsigned char message[] = "cannot divide by zero"; const unsigned char code[] = "XEN003"; const unsigned char name[] = "Division by Zero"; xenith_error_raise(message, sizeof(message) - 1, code, sizeof(code) - 1, name, sizeof(name) - 1); return 0; } if (left == INT64_MIN && right == -1) { const unsigned char message[] = "integer overflow"; const unsigned char code[] = "XEN017"; const unsigned char name[] = "Integer Overflow"; xenith_error_raise(message, sizeof(message) - 1, code, sizeof(code) - 1, name, sizeof(name) - 1); return 0; } return left / right; }
int64_t xenith_overflow_value(int64_t value, int8_t overflow) { if (overflow) { const unsigned char message[] = "integer overflow"; const unsigned char code[] = "XEN017"; const unsigned char name[] = "Integer Overflow"; xenith_error_raise(message, sizeof(message) - 1, code, sizeof(code) - 1, name, sizeof(name) - 1); return 0; } return value; }
const unsigned char *xenith_error_message(void) { return xenith_error.message; }
int64_t xenith_error_message_len(void) { return xenith_error.message_len; }
const unsigned char *xenith_error_code(void) { return xenith_error.code; }
int64_t xenith_error_code_len(void) { return xenith_error.code_len; }
const unsigned char *xenith_error_name(void) { return xenith_error.name; }
int64_t xenith_error_name_len(void) { return xenith_error.name_len; }
void xenith_echo(const unsigned char *data, int64_t length) {
    fwrite(data, 1, (size_t)length, stdout);
    fputc('\n', stdout);
}
void *xenith_closure_new(int64_t slots) { if (slots < 0 || slots > (INT64_MAX / (int64_t)sizeof(int64_t)) - 1) { xenith_allocation_failure(); return xenith_error_slot; } void *raw = calloc((size_t)slots + 1, sizeof(int64_t)); if (!raw) { xenith_allocation_failure(); return xenith_error_slot; } return raw; }
void xenith_closure_set(void *raw, int64_t slot, int64_t value) { if (!xenith_error_pending()) ((int64_t *)raw)[slot] = value; }
int64_t xenith_closure_get(void *raw, int64_t slot) { return xenith_error_pending() ? 0 : ((int64_t *)raw)[slot]; }
void xenith_string_concat(const unsigned char *a, int64_t a_len, const unsigned char *b, int64_t b_len, unsigned char **out, int64_t *out_len) { static unsigned char empty[] = {0}; if (a_len < 0 || b_len < 0 || a_len > INT64_MAX - b_len) { xenith_allocation_failure(); *out = empty; *out_len = 0; return; } *out_len = a_len + b_len; *out = *out_len ? malloc((size_t)*out_len) : empty; if (!*out && *out_len) { xenith_allocation_failure(); *out = empty; *out_len = 0; return; } memcpy(*out, a, (size_t)a_len); memcpy(*out + a_len, b, (size_t)b_len); }
int8_t xenith_string_equal(const unsigned char *a, int64_t a_len, const unsigned char *b, int64_t b_len) { return a_len == b_len && memcmp(a, b, (size_t)a_len) == 0; }
typedef struct { int64_t length, width; unsigned char data[]; } XenithList;
typedef struct { const unsigned char *key; int64_t key_len; unsigned char *value; } XenithMapEntry;
typedef struct { int64_t length, width; XenithMapEntry entries[]; } XenithMap;
static XenithList xenith_error_list = {0, 0, {0}};
static XenithMap xenith_error_map = {0, 0, {0}};
void *xenith_list_new(int64_t length, int64_t width) { if (length < 0 || width < 0 || (width && length > (INT64_MAX - (int64_t)sizeof(XenithList)) / width)) { xenith_allocation_failure(); return &xenith_error_list; } XenithList *list = calloc(1, sizeof(XenithList) + (size_t)length * (size_t)width); if (!list) { xenith_allocation_failure(); return &xenith_error_list; } list->length = length; list->width = width; return list; }
void *xenith_list_at(void *raw, int64_t index) { XenithList *list = raw; if (xenith_error_pending()) return xenith_error_slot; if (index < 0 || index >= list->length) { const unsigned char message[] = "index out of bounds"; const unsigned char code[] = "XEN004"; const unsigned char name[] = "Index Out of Bounds"; xenith_error_raise(message, sizeof(message) - 1, code, sizeof(code) - 1, name, sizeof(name) - 1); memset(xenith_error_slot, 0, sizeof(xenith_error_slot)); return xenith_error_slot; } return list->data + index * list->width; }
void *xenith_map_new(int64_t length, int64_t width) { if (length < 0 || width < 0 || length > (INT64_MAX - (int64_t)sizeof(XenithMap)) / (int64_t)sizeof(XenithMapEntry)) { xenith_allocation_failure(); return &xenith_error_map; } XenithMap *map = calloc(1, sizeof(XenithMap) + (size_t)length * sizeof(XenithMapEntry)); if (!map) { xenith_allocation_failure(); return &xenith_error_map; } map->length = length; map->width = width; return map; }
void xenith_map_put(void *raw, int64_t slot, const unsigned char *key, int64_t key_len, const void *value) { XenithMap *map = raw; if (xenith_error_pending() || slot < 0 || slot >= map->length) return; XenithMapEntry *entry = &map->entries[slot]; entry->key = key; entry->key_len = key_len; entry->value = malloc((size_t)map->width); if (!entry->value) { xenith_allocation_failure(); return; } memcpy(entry->value, value, (size_t)map->width); }
void *xenith_map_at(void *raw, const unsigned char *key, int64_t key_len) { XenithMap *map = raw; if (xenith_error_pending()) return xenith_error_slot; for (int64_t i = 0; i < map->length; i++) if (map->entries[i].key_len == key_len && !memcmp(map->entries[i].key, key, key_len)) return map->entries[i].value; const unsigned char message[] = "map key not found"; const unsigned char code[] = "XEN004"; const unsigned char name[] = "Index Out of Bounds"; xenith_error_raise(message, sizeof(message) - 1, code, sizeof(code) - 1, name, sizeof(name) - 1); memset(xenith_error_slot, 0, sizeof(xenith_error_slot)); return xenith_error_slot; }
void *xenith_bytes_new(const unsigned char *data, int64_t length) { XenithList *bytes = xenith_list_new(length, 1); if (xenith_error_pending()) return bytes; memcpy(bytes->data, data, (size_t)length); return bytes; }
int32_t xenith_list_len(void *raw) { return (int32_t)((XenithList *)raw)->length; }
int32_t xenith_map_len(void *raw) { return (int32_t)((XenithMap *)raw)->length; }
const unsigned char *xenith_map_key_at(void *raw, int64_t index) { XenithMap *map = raw; if (index < 0 || index >= map->length) { const unsigned char message[] = "index out of bounds"; const unsigned char code[] = "XEN004"; const unsigned char name[] = "Index Out of Bounds"; xenith_error_raise(message, sizeof(message) - 1, code, sizeof(code) - 1, name, sizeof(name) - 1); return (const unsigned char *)""; } return map->entries[index].key; }
int64_t xenith_map_key_len_at(void *raw, int64_t index) { XenithMap *map = raw; if (index < 0 || index >= map->length) { const unsigned char message[] = "index out of bounds"; const unsigned char code[] = "XEN004"; const unsigned char name[] = "Index Out of Bounds"; xenith_error_raise(message, sizeof(message) - 1, code, sizeof(code) - 1, name, sizeof(name) - 1); return 0; } return map->entries[index].key_len; }
void *xenith_list_append(void *raw, int64_t width, const void *value) { XenithList *old = raw; XenithList *list = xenith_list_new(old->length + 1, width); memcpy(list->data, old->data, old->length * width); memcpy(list->data + old->length * width, value, width); return list; }
int32_t xenith_map_remove(void *raw, const unsigned char *key, int64_t key_len) { XenithMap *map = raw; for (int64_t i = 0; i < map->length; i++) if (map->entries[i].key_len == key_len && !memcmp(map->entries[i].key, key, key_len)) { free(map->entries[i].value); for (int64_t j = i + 1; j < map->length; j++) map->entries[j - 1] = map->entries[j]; map->length -= 1; return 1; } return 0; }
int8_t xenith_map_contains(void *raw, const unsigned char *key, int64_t key_len) { XenithMap *map = raw; for (int64_t i = 0; i < map->length; i++) if (map->entries[i].key_len == key_len && !memcmp(map->entries[i].key, key, key_len)) return 1; return 0; }
"#;

pub fn write_source(directory: &Path) -> std::io::Result<PathBuf> {
    let path = directory.join("xenith-runtime.c");
    fs::write(&path, SOURCE)?;
    Ok(path)
}
