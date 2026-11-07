#include <stdio.h>
#include <stdlib.h>

struct sample {
	short *pcm;
	size_t count;
	FILE *handle;
};

static size_t read_pcm(struct sample *sample, size_t count) {
	return fread(sample->pcm, sizeof(short), count, sample->handle);
}

struct sample *sample_load(const char *name, size_t count) {
	struct sample *sample = calloc(1, sizeof(struct sample));
	if (sample == NULL) {
		return NULL;
	}
	sample->handle = fopen(name, "rb");
	if (sample->handle == NULL) {
		fprintf(stderr, "sample: cannot open %s\n", name);
		free(sample);
		return NULL;
	}
	sample->pcm = malloc(count * sizeof(short));
	if (sample->pcm == NULL) {
		fprintf(stderr, "sample: out of memory for %zu samples\n", count);
		fclose(sample->handle);
		free(sample);
		return NULL;
	}
	sample->count = read_pcm(sample, count);
	if (sample->count != count) {
		fprintf(stderr, "sample: short read of %s\n", name);
		free(sample->pcm);
		fclose(sample->handle);
		free(sample);
		return NULL;
	}
	return sample;
}

int sample_peak(const struct sample *sample) {
	int peak = 0;
	for (size_t index = 0; index < sample->count; index++) {
		int magnitude = sample->pcm[index] < 0 ? -sample->pcm[index] : sample->pcm[index];
		if (magnitude > peak) {
			peak = magnitude;
		}
	}
	return peak;
}
