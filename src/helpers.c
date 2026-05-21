// helpers.c — Thin C accessors for assist_extras and assist_ephem fields.

#include "rebound.h"
#include "assist.h"
#include "spk.h"

// Reset the ephemeris cache to "all slots invalid (-1e306)". Emulates the
// assist_init cache-initialization behavior between propagations.
//
// WHY THIS MATTERS FOR PERFORMANCE: `assist_all_ephem` does a 7-slot LRU on
// the cache; when cache is fresh (all slots = -1e306), the "find oldest" loop
// always picks slot 0 (single branch), but when slots are populated with
// stale-but-realistic t values from a previous propagation, the loop does real
// comparisons every iteration. Over ~7000 ephem lookups per 30-day Ceres-type
// integrate, this compounds to ~190 µs of overhead — the full PropagatorPool
// vs assist_propagate regression. Invalidating the cache between pool
// propagations restores fresh-sim performance.
void assist_rs_ephem_cache_reset(struct assist_extras* ax) {
    if (!ax || !ax->ephem_cache || !ax->ephem_cache->t) return;
    int N_total = ASSIST_BODY_NPLANETS;
    if (ax->ephem && ax->ephem->spk_asteroids) {
        N_total += ax->ephem->spk_asteroids->num;
    }
    for (int i = 0; i < 7 * N_total; i++) {
        ax->ephem_cache->t[i] = -1e306;
    }
}

// --- assist_extras field accessors ---

int assist_rs_extras_get_forces(const struct assist_extras* ax) { return ax->forces; }
void assist_rs_extras_set_forces(struct assist_extras* ax, int f) { ax->forces = f; }

int assist_rs_extras_get_geocentric(const struct assist_extras* ax) { return ax->geocentric; }
void assist_rs_extras_set_geocentric(struct assist_extras* ax, int g) { ax->geocentric = g; }

double* assist_rs_extras_get_particle_params(const struct assist_extras* ax) {
    return ax->particle_params;
}
void assist_rs_extras_set_particle_params(struct assist_extras* ax, double* p) {
    ax->particle_params = p;
}

// --- assist_ephem field accessors ---

double assist_rs_ephem_get_jd_ref(const struct assist_ephem* ephem) { return ephem->jd_ref; }
void   assist_rs_ephem_set_jd_ref(struct assist_ephem* ephem, double jd) { ephem->jd_ref = jd; }

double assist_rs_ephem_get_au(const struct assist_ephem* ephem) { return ephem->AU; }
double assist_rs_ephem_get_clight(const struct assist_ephem* ephem) { return ephem->CLIGHT; }
double assist_rs_ephem_get_c_au_per_day(const struct assist_ephem* ephem) { return ephem->c_AU_per_day; }
double assist_rs_ephem_get_re(const struct assist_ephem* ephem) { return ephem->RE; }
double assist_rs_ephem_get_re_eq(const struct assist_ephem* ephem) { return ephem->Re_eq; }
double assist_rs_ephem_get_emrat(const struct assist_ephem* ephem) { return ephem->EMRAT; }

// --- assist_extras non-gravitational force parameters ---

double assist_rs_extras_get_alpha(const struct assist_extras* ax) { return ax->alpha; }
void   assist_rs_extras_set_alpha(struct assist_extras* ax, double v) { ax->alpha = v; }

double assist_rs_extras_get_nk(const struct assist_extras* ax) { return ax->nk; }
void   assist_rs_extras_set_nk(struct assist_extras* ax, double v) { ax->nk = v; }

double assist_rs_extras_get_nm(const struct assist_extras* ax) { return ax->nm; }
void   assist_rs_extras_set_nm(struct assist_extras* ax, double v) { ax->nm = v; }

double assist_rs_extras_get_nn(const struct assist_extras* ax) { return ax->nn; }
void   assist_rs_extras_set_nn(struct assist_extras* ax, double v) { ax->nn = v; }

double assist_rs_extras_get_r0(const struct assist_extras* ax) { return ax->r0; }
void   assist_rs_extras_set_r0(struct assist_extras* ax, double v) { ax->r0 = v; }
