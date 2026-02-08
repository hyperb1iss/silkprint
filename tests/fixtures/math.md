---
title: Math Showcase
author: SilkPrint Test Suite
---

# Mathematics

All math in SilkPrint uses **Typst-native** syntax, not LaTeX.

## Inline Math

The Pythagorean theorem states that $x^2 + y^2 = z^2$ for a right triangle.

The sum of the first *n* natural numbers is $sum_(i=1)^n i = (n(n+1))/2$.

Euler's identity: $e^(i pi) + 1 = 0$.

The golden ratio is $phi = (1 + sqrt(5))/2 approx 1.618$.

## Display Math

The quadratic formula:

$ x = (-b plus.minus sqrt(b^2 - 4a c)) / (2a) $

An integral:

$ integral_0^infinity e^(-x) dif x = 1 $

A summation:

$ sum_(k=0)^infinity x^k / k! = e^x $

## Fractions

Inline fraction: $a/b$ and $(x+1)/(x-1)$.

Display fraction:

$ (d f)/(d x) = lim_(h -> 0) (f(x+h) - f(x)) / h $

## Matrices

A 2x2 matrix:

$ mat(a, b; c, d) $

A 3x3 identity matrix:

$ mat(1, 0, 0; 0, 1, 0; 0, 0, 1) $

## Subscripts and Superscripts

Variables: $x_1, x_2, ..., x_n$.

Combined: $a_i^2 + b_j^2 = c_(i j)^2$.

## Greek Letters and Symbols

Common symbols: $alpha, beta, gamma, delta, epsilon, theta, lambda, mu, pi, sigma, omega$.

Operators: $plus.minus, times, div, eq.not, lt.eq, gt.eq, approx, prop$.

Arrows: $arrow.r, arrow.l, arrow.r.double, arrow.l.r$.

Sets: $in, in.not, subset, union, sect, emptyset$.

## Multi-line Equations

$ f(x) &= x^2 + 2x + 1 \
       &= (x + 1)^2 $
