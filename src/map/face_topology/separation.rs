//! Profile separation contract: documented minimum-average gaps between
//! consecutive deformation profiles.
//!
//! Separation is checked independently of the per-profile visual bands in
//! `acceptance.rs`. The gap values are aspirational targets derived from the
//! measured matrix (see the struct docs for the current status).
use crate::map::face_topology::profiles::HexDeformationProfile;

/// Minimum average-displacement gaps between consecutive profiles.
///
/// Values are aspirational targets derived from measured margins: on the
/// canonical 40x40 the organic-to-Pagonia average gap is observed in the
/// 0.021-0.033 range, so `0.015` leaves headroom; the subtle-to-organic gap is
/// **not** currently met (Organic average is below Subtle's) and tuning is an
/// explicit follow-up, not part of this commit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileSeparationCriteria {
    pub subtle_to_organic_min_average_gap_ratio: f32,
    pub organic_to_pagonia_min_average_gap_ratio: f32,
}

impl ProfileSeparationCriteria {
    #[must_use]
    pub const fn for_defaults() -> Self {
        Self {
            subtle_to_organic_min_average_gap_ratio: 0.015,
            organic_to_pagonia_min_average_gap_ratio: 0.015,
        }
    }

    #[must_use]
    pub const fn min_gap_between(
        self,
        lower: HexDeformationProfile,
        upper: HexDeformationProfile,
    ) -> Option<f32> {
        match (lower, upper) {
            (HexDeformationProfile::Subtle, HexDeformationProfile::Organic) => {
                Some(self.subtle_to_organic_min_average_gap_ratio)
            }
            (HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike) => {
                Some(self.organic_to_pagonia_min_average_gap_ratio)
            }
            _ => None,
        }
    }
}

/// A documented average-separation gap that the measured outputs violate.
#[derive(Debug, Clone, PartialEq)]
pub enum ProfileSeparationViolation {
    SubtleNotBelowOrganic {
        subtle_average: f32,
        organic_average: f32,
        min_gap: f32,
    },
    OrganicNotBelowPagonia {
        organic_average: f32,
        pagonia_average: f32,
        min_gap: f32,
    },
}

/// Pure separation checker: every violated gap is returned.
///
/// # Parameters
/// - `criteria`: the documented minimum gaps.
/// - The three measured average displacement ratios (relative to `HEX_SIZE`).
#[must_use]
pub fn check_profile_separation(
    criteria: ProfileSeparationCriteria,
    subtle_average: f32,
    organic_average: f32,
    pagonia_average: f32,
) -> Vec<ProfileSeparationViolation> {
    let mut violations = Vec::new();
    if organic_average < subtle_average + criteria.subtle_to_organic_min_average_gap_ratio {
        violations.push(ProfileSeparationViolation::SubtleNotBelowOrganic {
            subtle_average,
            organic_average,
            min_gap: criteria.subtle_to_organic_min_average_gap_ratio,
        });
    }
    if pagonia_average < organic_average + criteria.organic_to_pagonia_min_average_gap_ratio {
        violations.push(ProfileSeparationViolation::OrganicNotBelowPagonia {
            organic_average,
            pagonia_average,
            min_gap: criteria.organic_to_pagonia_min_average_gap_ratio,
        });
    }
    violations
}
