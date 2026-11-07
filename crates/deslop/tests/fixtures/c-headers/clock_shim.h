#ifndef CLOCK_SHIM_H
#define CLOCK_SHIM_H

long clock_now(long offset);
long clock_elapsed(long start);
long clock_round(long ticks);
long clock_scale(long ticks);
long clock_add(long left, long right);
int clock_compare(int left, int right);
long clock_shift(long ticks, int places);

#endif
