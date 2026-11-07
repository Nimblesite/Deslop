// Package sample exercises the C normalisation rules: identifier
// collapse (identifier, field_identifier, type_identifier), literal
// collapse (int, float, char with an escape sequence, string with
// concatenation, true, false, NULL), comment drop, and the structural
// forms most likely to shift between tree-sitter-c patch releases.
//
// The C-specific half is C's own identity: object and function-like
// macros (with a line continuation), conditional compilation, typedef'd
// struct/enum/union, pointer-to-function types feeding a
// designated-initialiser dispatch table, compound literals, casts,
// sizeof, goto cleanup, do/while, the comma operator, ternaries,
// numeric literals in hex/float/suffix forms, and the storage-class
// spread of static/extern/const/volatile. Each is a plausible target
// for a grammar patch release, so each is pinned here.

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

// The quoted form needs a real file on disk: sample_local.h pins the
// include-guard pattern and contributes SAMPLE_EPOCH_OFFSET.
#include "sample_local.h"

// SAMPLE_LIMIT pins an object-like macro; SAMPLE_CLAMP pins a
// function-like macro with a line continuation.
#define SAMPLE_LIMIT 64
#define SAMPLE_CLAMP(value, low, high) \
	((value) < (low) ? (low) : ((value) > (high) ? (high) : (value)))

// Conditional compilation pins the #ifdef/#else pair; exactly one
// branch survives parsing.
#ifdef SAMPLE_TIGHT
#define SAMPLE_MARGIN 1
#else
#define SAMPLE_MARGIN 4
#endif

/* Block comments are dropped too, including above declarations. */

// sample_point pins struct field_identifier collapse.
typedef struct sample_point {
	int x;
	int y;
} sample_point;

// sample_phase pins enum_constant collapse.
typedef enum sample_phase {
	PHASE_IDLE,
	PHASE_RUNNING,
	PHASE_DONE
} sample_phase;

// sample_value pins union arms.
typedef union sample_value {
	int as_int;
	float as_float;
	const char *as_text;
} sample_value;

// sample_metric pins pointer-to-function types; the dispatch table in
// sample_render consumes it through designated initialisers.
typedef int (*sample_metric)(const sample_point *point);

// Storage classes: static file-scope state, const, volatile, and the
// extern epoch defined in another translation unit.
static const int sample_margin = SAMPLE_MARGIN;
static volatile int sample_ticks = 0;
extern const int sample_epoch;

/* Idle metric: a hexadecimal mask, a shift, a float literal with a
 * suffix, a cast, and the offset contributed by the quoted header. */
static int sample_measure_idle(const sample_point *point) {
	const int masked = (point->x & 0xFFu) << 2;
	return masked + (int)(2.5f * sample_margin)
		+ sample_epoch * SAMPLE_EPOCH_OFFSET;
}

/* Running metric: do/while so the post-conditioned loop shape is
 * pinned, with a ternary deciding the increment. */
static int sample_measure_running(const sample_point *point) {
	int steps = 0;
	int cursor = point->y;
	do {
		cursor -= 3;
		steps += cursor > 0 ? 1 : 2;
	} while (cursor > 0);
	return (int)(steps * 1.5e0) * 2u;
}

/* Done metric: switch/case with fallthrough, a char literal with an
 * escape, string literal concatenation, and a suffix literal. */
static int sample_measure_done(const sample_point *point) {
	switch (point->x) {
	case 0:
		sample_ticks += 1u;
		/* fall through */
	case 1:
		return (int)strlen("sample " "done") + '\t';
	default:
		return point->x * 100UL;
	}
}

// sample_render walks the dispatch table, builds a point through a
// compound literal, clamps through the function macro, formats inside
// a comma-operator loop, and bails through goto cleanup.
bool sample_render(const sample_point *points, int count, char *out, int out_size) {
	bool wrote = false;
	if (points == NULL || out == NULL || count < 0) {
		return wrote;
	}
	static const sample_metric dispatch[PHASE_DONE + 1] = {
		[PHASE_IDLE] = sample_measure_idle,
		[PHASE_RUNNING] = sample_measure_running,
		[PHASE_DONE] = sample_measure_done,
	};
	int written = 0;
	for (int index = 0, stride = 1; index < count && written < out_size - 1;
		index++, stride++) {
		const sample_metric metric = dispatch[index % (PHASE_DONE + 1)];
		if (metric == NULL) {
			goto cleanup;
		}
		const sample_value blend = {.as_int = SAMPLE_CLAMP(points[index].x, 0, sample_margin * 8)};
		const sample_point offset = (sample_point){.x = blend.as_int, .y = points[index].y};
		written += snprintf(out + written, out_size - written,
			"stride %d measured %d\n", stride, metric(&offset) + (int)sizeof(offset));
		wrote = true;
	}
cleanup:
	return wrote;
}
