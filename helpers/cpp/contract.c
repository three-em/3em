#ifdef __cplusplus
extern "C" {
#endif

#include <stdlib.h>
#include "cJSON/cJSON.h"

int LEN = 0;

__attribute__((used)) int* _alloc(size_t size) {
  int *ptr = malloc(size);
  return ptr;
}

__attribute__((used)) int get_len() {
  return LEN;
}

__attribute__((used)) char* handle(
  char* state_ptr,
  int state_len,
  char* action_ptr,
  int action_len
) {
  cJSON *state = NULL;
  cJSON *action = NULL;

  state = cJSON_ParseWithLength(state_ptr, state_len);
  action = cJSON_ParseWithLength(action_ptr, action_len);

  cJSON *result = cJSON_CreateObject();
  LEN = sizeof(result->string);
  return result->string;
}

#ifdef __cplusplus
}
#endif
