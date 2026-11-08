#ifndef HOOKS_H
#define HOOKS_H

int alpha_open(const char *path, int flags);
void beta_report(const char *message);
long gamma_seek(int handle, long offset);
const char *delta_name(int code);

#endif
