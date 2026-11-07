#include <stddef.h>

int (*on_open)(const char *path, int flags);
int (*on_close)(int handle);
void (*on_error)(const char *message, int code);
long (*on_seek)(int handle, long offset, int whence);

int alpha_open(const char *path, int flags) {
	if (on_open == NULL) {
		return -1;
	}
	return on_open(path, flags);
}
