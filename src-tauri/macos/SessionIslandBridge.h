#pragma once

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*SmalltalkIslandActionCallback)(const char *action_json);

void smalltalk_island_init(void);
void smalltalk_island_set_action_callback(SmalltalkIslandActionCallback callback);
void smalltalk_island_update_json(const char *json);
void smalltalk_island_show(void);
void smalltalk_island_hide(void);
void smalltalk_island_set_expanded(bool expanded);
void smalltalk_island_reposition(void);
void smalltalk_island_shutdown(void);

#ifdef __cplusplus
}
#endif
