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
void xenith_error_clear(void) { free(xenith_error.message); xenith_error = (XenithError){0, NULL, 0, NULL, 0, NULL, 0}; }
int8_t xenith_error_pending(void) { return xenith_error.pending; }
void xenith_error_raise(const unsigned char *message, int64_t message_len, const unsigned char *code, int64_t code_len, const unsigned char *name, int64_t name_len) { xenith_error_clear(); xenith_error.pending = 1; xenith_error.message = malloc((size_t)message_len); memcpy(xenith_error.message, message, (size_t)message_len); xenith_error.message_len = message_len; xenith_error.code = code; xenith_error.code_len = code_len; xenith_error.name = name; xenith_error.name_len = name_len; }
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
void *xenith_closure_new(int64_t slots) { return calloc((size_t)slots + 1, sizeof(int64_t)); }
void xenith_closure_set(void *raw, int64_t slot, int64_t value) { ((int64_t *)raw)[slot] = value; }
int64_t xenith_closure_get(void *raw, int64_t slot) { return ((int64_t *)raw)[slot]; }
void xenith_string_concat(const unsigned char *a, int64_t a_len, const unsigned char *b, int64_t b_len, unsigned char **out, int64_t *out_len) { *out_len = a_len + b_len; *out = malloc((size_t)*out_len); memcpy(*out, a, (size_t)a_len); memcpy(*out + a_len, b, (size_t)b_len); }
int8_t xenith_string_equal(const unsigned char *a, int64_t a_len, const unsigned char *b, int64_t b_len) { return a_len == b_len && memcmp(a, b, (size_t)a_len) == 0; }
typedef struct { int64_t length, width; unsigned char data[]; } XenithList;
typedef struct { const unsigned char *key; int64_t key_len; unsigned char *value; } XenithMapEntry;
typedef struct { int64_t length, width; XenithMapEntry entries[]; } XenithMap;
void *xenith_list_new(int64_t length, int64_t width) { XenithList *list = calloc(1, sizeof(XenithList) + length * width); list->length = length; list->width = width; return list; }
void *xenith_list_at(void *raw, int64_t index) { XenithList *list = raw; if (index < 0 || index >= list->length) abort(); return list->data + index * list->width; }
void *xenith_map_new(int64_t length, int64_t width) { XenithMap *map = calloc(1, sizeof(XenithMap) + length * sizeof(XenithMapEntry)); map->length = length; map->width = width; return map; }
void xenith_map_put(void *raw, int64_t slot, const unsigned char *key, int64_t key_len, const void *value) { XenithMap *map = raw; XenithMapEntry *entry = &map->entries[slot]; entry->key = key; entry->key_len = key_len; entry->value = malloc(map->width); memcpy(entry->value, value, map->width); }
void *xenith_map_at(void *raw, const unsigned char *key, int64_t key_len) { XenithMap *map = raw; for (int64_t i = 0; i < map->length; i++) if (map->entries[i].key_len == key_len && !memcmp(map->entries[i].key, key, key_len)) return map->entries[i].value; abort(); }
void *xenith_bytes_new(const unsigned char *data, int64_t length) { XenithList *bytes = xenith_list_new(length, 1); memcpy(bytes->data, data, length); return bytes; }
int32_t xenith_list_len(void *raw) { return (int32_t)((XenithList *)raw)->length; }
int32_t xenith_map_len(void *raw) { return (int32_t)((XenithMap *)raw)->length; }
const unsigned char *xenith_map_key_at(void *raw, int64_t index) { XenithMap *map = raw; if (index < 0 || index >= map->length) abort(); return map->entries[index].key; }
int64_t xenith_map_key_len_at(void *raw, int64_t index) { XenithMap *map = raw; if (index < 0 || index >= map->length) abort(); return map->entries[index].key_len; }
void *xenith_list_append(void *raw, int64_t width, const void *value) { XenithList *old = raw; XenithList *list = xenith_list_new(old->length + 1, width); memcpy(list->data, old->data, old->length * width); memcpy(list->data + old->length * width, value, width); return list; }
int32_t xenith_map_remove(void *raw, const unsigned char *key, int64_t key_len) { XenithMap *map = raw; for (int64_t i = 0; i < map->length; i++) if (map->entries[i].key_len == key_len && !memcmp(map->entries[i].key, key, key_len)) { free(map->entries[i].value); for (int64_t j = i + 1; j < map->length; j++) map->entries[j - 1] = map->entries[j]; map->length -= 1; return 1; } return 0; }
int8_t xenith_map_contains(void *raw, const unsigned char *key, int64_t key_len) { XenithMap *map = raw; for (int64_t i = 0; i < map->length; i++) if (map->entries[i].key_len == key_len && !memcmp(map->entries[i].key, key, key_len)) return 1; return 0; }
"#;

pub fn write_source(directory: &Path) -> std::io::Result<PathBuf> {
    let path = directory.join("xenith-runtime.c");
    fs::write(&path, SOURCE)?;
    Ok(path)
}
