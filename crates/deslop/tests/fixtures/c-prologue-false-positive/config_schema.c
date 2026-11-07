#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SERVICE_TAG "indexer"
#define MAX_ENTRIES 512
#define RETRY_CEILING 30

struct schema_field {
	const char *name;
	int required;
	const char *fallback;
};

void describe_schema(const struct schema_field *fields, int field_count, char *out, int out_size) {
	int written = 0;
	for (int index = 0; index < field_count; index++) {
		const struct schema_field *field = &fields[index];
		const char *marker = field->required != 0 ? "!" : "?";
		if (written >= out_size - 1) {
			break;
		}
		if (field->name == NULL || field->name[0] == '\0') {
			written += snprintf(out + written, out_size - written,
				"<anonymous:%s>", marker);
		} else {
			written += snprintf(out + written, out_size - written,
				"%s%s=%s", marker, field->name, field->fallback);
		}
		if (index + 1 < field_count) {
			out[written] = ',';
			written++;
		}
	}
	out[written] = '\0';
}
