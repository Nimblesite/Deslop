#ifndef MATH_SHIM_H
#define MATH_SHIM_H

double shim_sqrt(double x);
double shim_exp(double x);
double shim_log(double x);
double shim_tanh(double x);
double shim_pow(double base, double exponent);
float shim_fmodf(float x, float y);
double shim_ldexp(double x, int exponent);

#endif
