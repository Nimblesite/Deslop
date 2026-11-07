#include <stddef.h>

int (*on_open)(const char *path, int flags);
int (*on_close)(int handle);
void (*on_error)(const char *message, int code);
long (*on_seek)(int handle, long offset, int whence);

void beta_report(const char *message) {
	for (int attempt = 0; attempt < 3; attempt++) {
		if (on_error != NULL) {
			on_error(message, attempt);
		}
	}
}
