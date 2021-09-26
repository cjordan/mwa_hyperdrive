// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#include "common.cuh"
#include "memory.h"

Addresses init_model(int num_baselines, int num_freqs, int num_tiles, int sbf_l, int sbf_n, FLOAT sbf_c, FLOAT sbf_dx,
                     UVW *uvws, FLOAT *freqs, FLOAT *shapelet_basis_values, void *d_fee_coeffs, int num_fee_beam_coeffs,
                     int num_unique_fee_tiles, int num_unique_fee_freqs, uint64_t *d_beam_jones_map,
                     void *d_beam_norm_jones, JonesF32 *vis) {
    UVW *d_uvws = NULL;
    size_t size_uvws = num_baselines * sizeof(UVW);
    cudaSoftCheck(cudaMalloc(&d_uvws, size_uvws));
    cudaSoftCheck(cudaMemcpy(d_uvws, uvws, size_uvws, cudaMemcpyHostToDevice));

    FLOAT *d_freqs = NULL;
    size_t size_freqs = num_freqs * sizeof(FLOAT);
    cudaSoftCheck(cudaMalloc(&d_freqs, size_freqs));
    cudaSoftCheck(cudaMemcpy(d_freqs, freqs, size_freqs, cudaMemcpyHostToDevice));

    FLOAT *d_shapelet_basis_values = NULL;
    size_t size_sbfs = sbf_l * sbf_n * sizeof(FLOAT);
    cudaSoftCheck(cudaMalloc(&d_shapelet_basis_values, size_sbfs));
    cudaSoftCheck(cudaMemcpy(d_shapelet_basis_values, shapelet_basis_values, size_sbfs, cudaMemcpyHostToDevice));

    JonesF32 *d_vis = NULL;
    int num_vis = num_baselines * num_freqs;
    size_t size_vis = num_vis * sizeof(JonesF32);
    cudaSoftCheck(cudaMalloc(&d_vis, size_vis));
    cudaSoftCheck(cudaMemcpy(d_vis, vis, size_vis, cudaMemcpyHostToDevice));

    return Addresses{.num_freqs = num_freqs,
                     .num_vis = num_vis,
                     .num_tiles = num_tiles,
                     .sbf_l = sbf_l,
                     .sbf_n = sbf_n,
                     .sbf_c = sbf_c,
                     .sbf_dx = sbf_dx,
                     .d_uvws = d_uvws,
                     .d_freqs = d_freqs,
                     .d_shapelet_basis_values = d_shapelet_basis_values,
                     .d_fee_coeffs = d_fee_coeffs,
                     .num_fee_beam_coeffs = num_fee_beam_coeffs,
                     .num_unique_fee_tiles = num_unique_fee_tiles,
                     .num_unique_fee_freqs = num_unique_fee_freqs,
                     .d_beam_jones_map = d_beam_jones_map,
                     .d_beam_norm_jones = d_beam_norm_jones,
                     .d_vis = d_vis,
                     .host_vis = vis};
}

void copy_vis(const Addresses *a) {
    cudaSoftCheck(cudaMemcpy(a->host_vis, a->d_vis, a->num_vis * sizeof(JonesF32), cudaMemcpyDeviceToHost));
}

void clear_vis(Addresses *a) { cudaMemset(a->d_vis, 0.0, a->num_vis * sizeof(JonesF32)); }

void destroy(Addresses *a) {
    cudaSoftCheck(cudaFree(a->d_uvws));
    cudaSoftCheck(cudaFree(a->d_freqs));
    cudaSoftCheck(cudaFree(a->d_shapelet_basis_values));
    cudaSoftCheck(cudaFree(a->d_vis));
}
