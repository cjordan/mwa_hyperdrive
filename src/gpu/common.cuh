// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#pragma once

#include <assert.h>
#include <stdio.h>

#include "types.h"

// HIP-specific defines.
#if __HIPCC__
#define gpuMalloc             hipMalloc
#define gpuFree               hipFree
#define gpuMemcpy             hipMemcpy
#define gpuMemcpyHostToDevice hipMemcpyHostToDevice
#define gpuGetErrorString     hipGetErrorString
#define gpuGetLastError       hipGetLastError
#define gpuDeviceSynchronize  hipDeviceSynchronize
#define gpuError_t            hipError_t
#define gpuSuccess            hipSuccess

// If SINGLE is enabled, use single-precision floats everywhere. Otherwise
// default to double-precision.
#ifdef SINGLE
#define FLOAT4       float4
#define SINCOS       sincosf
#define EXP          expf
#define POW          powf
#define FLOOR        floorf
#define COMPLEX      hipFloatComplex
#define MAKE_COMPLEX make_hipFloatComplex
#define CUCONJ       hipConjf
#define LOG          logf
#define EXP          expf
#else
#define FLOAT4       double4
#define SINCOS       sincos
#define EXP          exp
#define POW          pow
#define FLOOR        floor
#define COMPLEX      hipDoubleComplex
#define MAKE_COMPLEX make_hipDoubleComplex
#define CUCONJ       hipConj
#define LOG          log
#define EXP          exp
#endif // SINGLE

// CUDA-specific defines.
#elif __CUDACC__

#define gpuMalloc             cudaMalloc
#define gpuFree               cudaFree
#define gpuMemcpy             cudaMemcpy
#define gpuMemcpyHostToDevice cudaMemcpyHostToDevice
#define gpuGetErrorString     cudaGetErrorString
#define gpuGetLastError       cudaGetLastError
#define gpuDeviceSynchronize  cudaDeviceSynchronize
#define gpuError_t            cudaError_t
#define gpuSuccess            cudaSuccess
#define warpSize              32

#ifdef SINGLE
#define FLOAT4       float4
#define SINCOS       sincosf
#define EXP          expf
#define POW          powf
#define FLOOR        floorf
#define COMPLEX      cuFloatComplex
#define MAKE_COMPLEX make_cuFloatComplex
#define CUCONJ       cuConjf
#define LOG          logf
#define EXP          expf
#else
#define FLOAT4       double4
#define SINCOS       sincos
#define EXP          exp
#define POW          pow
#define FLOOR        floor
#define COMPLEX      cuDoubleComplex
#define MAKE_COMPLEX make_cuDoubleComplex
#define CUCONJ       cuConj
#define LOG          log
#define EXP          exp
#endif // SINGLE
#endif // __HIPCC__
// #define C32 cuFloatComplex
// #define C64 cuDoubleComplex

#ifdef __CUDACC__
#include <cuComplex.h>
#elif __HIPCC__
#include <hip/hip_complex.h>
#include <hip/hip_runtime.h>
#endif

const FLOAT VEL_C = 299792458.0;                           // speed of light in a vacuum
const FLOAT LN_2 = 0.6931471805599453;                     // ln(2), or log_e(2)
const FLOAT TAU = 6.283185307179586;                       // 2 * PI
const FLOAT FRAC_PI_2 = 1.5707963267948966;                // PI / 2
const FLOAT SQRT_FRAC_PI_SQ_2_LN_2 = 2.6682231283184983;   // sqrt( PI^2 / (2 ln(2)) )
const FLOAT EXP_CONST = -((FRAC_PI_2 * FRAC_PI_2) / LN_2); // -( (PI/2)^2 / ln(2) )
const FLOAT SHAPELET_CONST = SQRT_FRAC_PI_SQ_2_LN_2 / SBF_DX;

typedef struct JONES_C {
    // The (0,0) component
    COMPLEX j00;
    // The (0,1) component
    COMPLEX j01;
    // The (1,0) component
    COMPLEX j10;
    // The (1,1) component
    COMPLEX j11;
} JONES_C;

inline __device__ COMPLEX operator+(const COMPLEX a, const COMPLEX b) { return MAKE_COMPLEX(a.x + b.x, a.y + b.y); }

inline __device__ COMPLEX operator*(const COMPLEX a, const COMPLEX b) {
    return MAKE_COMPLEX(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

inline __device__ void operator*=(COMPLEX &a, const COMPLEX b) {
    a = MAKE_COMPLEX(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

inline __device__ COMPLEX operator*(const COMPLEX a, const FLOAT b) { return MAKE_COMPLEX(a.x * b, a.y * b); }

inline __device__ void operator+=(COMPLEX &a, const COMPLEX b) {
    a.x += b.x;
    a.y += b.y;
}

inline __device__ JONES operator*(const JONES a, const FLOAT b) {
    return JONES{
        .j00_re = a.j00_re * b,
        .j00_im = a.j00_im * b,
        .j01_re = a.j01_re * b,
        .j01_im = a.j01_im * b,
        .j10_re = a.j10_re * b,
        .j10_im = a.j10_im * b,
        .j11_re = a.j11_re * b,
        .j11_im = a.j11_im * b,
    };
}

inline __device__ JONES operator*(const JONES a, const COMPLEX b) {
    return JONES{
        .j00_re = a.j00_re * b.x - a.j00_im * b.y,
        .j00_im = a.j00_re * b.y + a.j00_im * b.x,
        .j01_re = a.j01_re * b.x - a.j01_im * b.y,
        .j01_im = a.j01_re * b.y + a.j01_im * b.x,
        .j10_re = a.j10_re * b.x - a.j10_im * b.y,
        .j10_im = a.j10_re * b.y + a.j10_im * b.x,
        .j11_re = a.j11_re * b.x - a.j11_im * b.y,
        .j11_im = a.j11_re * b.y + a.j11_im * b.x,
    };
}

inline __device__ void operator+=(JONES &a, const JONES b) {
    a.j00_re += b.j00_re;
    a.j00_im += b.j00_im;
    a.j01_re += b.j01_re;
    a.j01_im += b.j01_im;
    a.j10_re += b.j10_re;
    a.j10_im += b.j10_im;
    a.j11_re += b.j11_re;
    a.j11_im += b.j11_im;
}

inline __device__ void operator+=(JonesF32 &a, const JonesF64 b) {
    a.j00_re += (float)b.j00_re;
    a.j00_im += (float)b.j00_im;
    a.j01_re += (float)b.j01_re;
    a.j01_im += (float)b.j01_im;
    a.j10_re += (float)b.j10_re;
    a.j10_im += (float)b.j10_im;
    a.j11_re += (float)b.j11_re;
    a.j11_im += (float)b.j11_im;
}

inline __device__ UVW operator*(const UVW a, const FLOAT b) {
    return UVW{
        .u = a.u * b,
        .v = a.v * b,
        .w = a.w * b,
    };
}

inline __device__ UVW operator/(const UVW a, const FLOAT b) {
    return UVW{
        .u = a.u / b,
        .v = a.v / b,
        .w = a.w / b,
    };
}

inline __device__ ShapeletUV operator*(const ShapeletUV a, const FLOAT b) {
    return ShapeletUV{
        .u = a.u * b,
        .v = a.v * b,
    };
}

/**
 * Multiply a Jones matrix by two beam Jones matrices (i.e. J1 . J . J2^H).
 */
inline __device__ void apply_beam(const JONES *j1, JONES *j, const JONES *j2) {
    // Cast the input Jones matrices to complex forms for convenience.
    JONES_C *j1c = (JONES_C *)j1;
    JONES_C *jc = (JONES_C *)j;
    JONES_C *j2c = (JONES_C *)j2;

    // J1 . J
    JONES_C temp{
        .j00 = j1c->j00 * jc->j00 + j1c->j01 * jc->j10,
        .j01 = j1c->j00 * jc->j01 + j1c->j01 * jc->j11,
        .j10 = j1c->j10 * jc->j00 + j1c->j11 * jc->j10,
        .j11 = j1c->j10 * jc->j01 + j1c->j11 * jc->j11,
    };

    // J2^H
    JONES_C j2h = JONES_C{
        .j00 = CUCONJ(j2c->j00),
        .j01 = CUCONJ(j2c->j10),
        .j10 = CUCONJ(j2c->j01),
        .j11 = CUCONJ(j2c->j11),
    };

    // (J1 . J) . J2^H
    jc->j00 = temp.j00 * j2h.j00 + temp.j01 * j2h.j10;
    jc->j01 = temp.j00 * j2h.j01 + temp.j01 * j2h.j11;
    jc->j10 = temp.j10 * j2h.j00 + temp.j11 * j2h.j10;
    jc->j11 = temp.j10 * j2h.j01 + temp.j11 * j2h.j11;
}
