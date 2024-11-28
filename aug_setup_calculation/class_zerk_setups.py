import numpy as np
import pandas as pd
from scipy import stats

WEIGHTS = {                 #  weight, min, max  # minimum required, maximum allowed
    "Capacity"              : [0.01, np.nan, np.nan],
    "Critical Hit Chance"   : [0.4, np.nan, np.nan],
    "Critical Hit Strength" : [0.4, np.nan, np.nan],
    "Damage"                : [0.7, 4.0, np.nan],
    "Docking Speed"         : [0.0, np.nan, np.nan],
    "Electric Tempering"    : [-0.7, np.nan, 0],
    "Energy Charge"         : [0.5, np.nan, np.nan],
    "Energy Max"            : [0.1, np.nan, np.nan],
    "Hostility"             : [0.5, np.nan, np.nan],
    "Inertial Dampening"    : [0.0, np.nan, np.nan],
    "Multifiring"           : [0.0, np.nan, np.nan],
    "Projectile Velocity"   : [0.0, np.nan, np.nan],
    "Radar"                 : [0.0, np.nan, np.nan],
    "Range"                 : [0.4, np.nan, np.nan],
    "Rate of Fire"          : [0.6, np.nan, np.nan],
    "Resistance to Damage"  : [1.0, np.nan, np.nan],
    "Shield Max"            : [1.0, np.nan, np.nan],
    "Shield Recovery"       : [0.1, np.nan, np.nan],
    "Speed"                 : [0.1, np.nan, np.nan],
    "Thrust"                : [0.01, np.nan, np.nan],
    "Tracking"              : [0.01, np.nan, np.nan],
    "Tractor Power"         : [0.0, np.nan, np.nan],
    "Tractor Range"         : [0.0, np.nan, np.nan],
    "Transference Power"    : [0.0, np.nan, np.nan],
    "Turning"               : [0.1, np.nan, np.nan],
    "Visibility"            : [0.0, np.nan, np.nan],
    "Weapon Hold"           : [0.1, np.nan, np.nan],
    "Weapons Slots"         : [0.05, np.nan, np.nan],
    "Weight"                : [0.0, np.nan, np.nan],
}


def compute_weighted_avg_zscore(df):
    # get min values
    min_vals = np.array([WEIGHTS[i][1] for i in df.columns])
    # filter
    for i, mv in enumerate(min_vals):
        if not np.isnan(mv):
            df = df.loc[df.iloc[:, i] >= mv]
    
    # get max values
    max_vals = np.array([WEIGHTS[i][2] for i in df.columns])
    # filter
    for i, mv in enumerate(max_vals):
        if not np.isnan(mv):
            df = df.loc[df.iloc[:, i] <= mv]

    # get weights, make sure they are same order as the columns
    weights = np.array([WEIGHTS[i][0] for i in df.columns])
    # normalize weights
    weights /= np.sum(weights)

    wavg_scores = np.nansum(df.values * weights, axis=1)

    # multiply by the # of columns contributing * the weights
    ncol_weight = np.sum((1 - np.isnan(df.values)) * weights, axis=1)

    return pd.Series(wavg_scores * ncol_weight, index=df.index)


def main():
    augs = pd.read_csv("augmenter_setups_3.csv", index_col=0)

    # replace 0s with nan
    augs = augs.replace(0.0, np.nan)

    # compute the z-score
    aug_zscore = augs.transform(stats.zscore, axis=0, nan_policy='omit')

    wavg_zscores = compute_weighted_avg_zscore(aug_zscore)

    print(wavg_zscores.sort_values(ascending=False))



if __name__ == "__main__":
    main()

