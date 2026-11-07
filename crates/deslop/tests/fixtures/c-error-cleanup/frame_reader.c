#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct frame {
	unsigned char *bytes;
	size_t length;
	FILE *source;
};

struct frame *frame_open(const char *path, size_t length) {
	struct frame *frame = malloc(sizeof(struct frame));
	if (frame == NULL) {
		return NULL;
	}
	frame->source = fopen(path, "rb");
	if (frame->source == NULL) {
		fprintf(stderr, "frame: cannot open %s\n", path);
		free(frame);
		return NULL;
	}
	frame->bytes = malloc(length);
	if (frame->bytes == NULL) {
		fprintf(stderr, "frame: out of memory for %zu bytes\n", length);
		fclose(frame->source);
		free(frame);
		return NULL;
	}
	frame->length = fread(frame->bytes, 1, length, frame->source);
	if (frame->length != length) {
		fprintf(stderr, "frame: short read of %s\n", path);
		free(frame->bytes);
		fclose(frame->source);
		free(frame);
		return NULL;
	}
	return frame;
}

void frame_close(struct frame *frame) {
	if (frame == NULL) {
		return;
	}
	free(frame->bytes);
	fclose(frame->source);
	free(frame);
}
