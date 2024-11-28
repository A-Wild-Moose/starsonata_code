import itertools
from timeit import default_timer as timer

import numpy as np
from scipy.special import comb as sp_comb
import pandas as pd
import tqdm


def sum_bonuses(mods):
    factors = np.full_like(mods, np.nan)

    factors[mods > 0] = mods[mods > 0]
    factors[mods <= 0] = -1 / (mods[mods <= 0] + 1) + 1

    # THIS IS value multiplied for actual calculation
    # if (res := np.nansum(factors)) > 0:
    #     return res + 1
    # else:
    #     return -1 / (res - 1)

    # SHIP VIEWER VALUE
    if (res := np.nansum(factors)) > 0:
        return res
    else:
        return -1 - 1 / (res - 1)


def sum_bonuses_2d(mods):
    factors = mods.copy()
    factors[factors <= 0] = -1 / (factors[factors <= 0] + 1) + 1

    res = np.nansum(factors, axis=0)
    res[res <= 0] = -1 - 1 / (res[res <= 0] - 1)

    return res


def main(aug_slots, min_tech=0, max_tech=50, excludes=None, includes=None, name_format=None):
    augs = pd.read_csv("parsed_augmenters.csv", index_col=0)

    if excludes is None:
        excludes = []

    # filter on tech level
    filt_augs = augs.loc[(augs['Tech'] >= min_tech) & (augs['Tech'] <= max_tech)]

    # filter on excludes
    for exc in excludes:
        filt_augs = filt_augs.loc[~filt_augs.index.str.contains(exc)]
    
    # add back any includes
    if includes is not None:
        filt_augs = pd.concat(
            (
                filt_augs,
                augs.loc[includes]
            ),
            ignore_index=False
        )
    
    # drop any na columns from filt_augs already
    filt_augs = filt_augs.dropna(axis=1, how='all')

    # pop the tech row
    filt_augs.pop("Tech")
    
    # n combinations
    n = filt_augs.shape[0]
    n_combs = sp_comb(n, aug_slots, repetition=True)

    combs = itertools.combinations_with_replacement(filt_augs.index, aug_slots)
    # comb_results = pd.DataFrame(index=range(int(n_combs)), columns=['Augs'] + list(filt_augs.columns[1:]))
    comb_results = pd.DataFrame(index=list(combs), columns=filt_augs.columns)
    
    with tqdm.tqdm(total=comb_results.shape[0]) as pbar:
        for i, _ in comb_results.iterrows():
            # dont drop na since just going to combine and have all the columns anyways
            subset = filt_augs.loc[list(i)]  #.dropna(axis=1, how='all')

            # comb_results.loc[[i], :] = subset.agg(sum_bonuses, axis=0)
            # this is MUCH faster than using agg
            comb_results.loc[[i], :] = sum_bonuses_2d(subset.values)

            pbar.update(1)


    if name_format is None:
        comb_results.to_csv(f"augmenter_setups_{aug_slots}.csv", index=True)
    else:
        comb_results.to_csv(name_format.format(aug_slots=aug_slots), index=True)


# if __name__ == "__main__":
#     """
#     No class bonuses, no ship bonuses. Aug Tweaking 5.5%

#     Sup hostile, sup shield, sup recov
#     Damage:          +78.07%
#     Energy Charge:   +12.66%
#     Hostility:       +78.91%
#     Shield Max:      +88.92%
#     Shield Recovery: -29.49%
#     """
#     main(
#         4,
#         min_tech=21,
#         excludes=['Sup. ', 'Ult. ', 'Demented ', 'Ship Mastery Augmenter', "Capital Offensive", "Capital Defensive", "Navigator's Offensive", "Navigator's Defensive"],
#         includes=[
#             "The Emperor's Augmenter",
#             "Adamanturized Defensive Augmenter",
#             "Adamanturized Invigorating Augmenter",
#             "Adamanturized Marksman Augmenter",
#             "Adamanturized Monkey Augmenter",
#             "Adamanturized Patrol Augmenter",
#             "Adamanturized Racer Augmenter",
#             "Adamanturized Rage Augmenter",
#             "Adamanturized Sardine Augmenter",
#         ]
#     )

#     # print(sum_bonuses(np.array([-0.75, -0.75, 1.0])))
#     # print(sum_bonuses(np.array([0.6, 0.6, -0.4])))

if __name__ == '__main__':
    main(
        4,
        min_tech=24,
        includes=[
            "Adamanturized Defensive Augmenter",
            "Adamanturized Rage Augmenter",
            "Ult. Artillery Augmenter",
            "Ult. Assassin Augmenter",
            "Ult. Barrage Augmenter",
            "La Buse Augmenter",
            "El Draque Augmenter",
            "Divine Wattage Augmenter",
            "Kidd's Modification",
            "Nightfury's Anger",
            "Nightfury's Patience",
            "Raging Aphrodite Augmenter",
            "Raging Dionysis Augmenter",
            "Grand Navigator's Augmenter",
            "Lunarian Augmenter",
            "Perilous Augmenter",
            "Selenite Augmenter",
            "Qa'ik Urk'qii Akk'oj",
            "Qa'ik Vazuk Akk'oj",
            "Ultimate Art of Engineering Augmenter",
            "The Emperor's Augmenter",
            "Ares Offensive Mode Augmenter",
            "Cunning of Hermes Augmenter",
            "Cunning of Hermes+ Augmenter",
            'Deimos Augmenter',
            "Hantr Psu",
            "Zarkara Sattva",
            "Vaidya Bhava",
            "Mechanical Hound Augmenter",
            "Twisted Death Augmenter",
            "Twisted Tesla Augmenter",
            "Celeste Mark Augmenter",
            "Celestial Nebulae Augmenter",
            "Celestial Rage Augmenter",
            "Eclipse Augmenter",
            "Empyreal Rage Augmenter",
            "Heavenly Foresight Augmenter",
            "Luminous Brace Levee Augmenter",
            "Luminous Intersticial Desistance Augmenter",
            "Luminous Munitions Amplifier Augmenter",
            "Luminous Resplendent Fluxion Augmenter",
            "Luminous Obliterative Integrity Augmenter",
            "Luminous Ossified Buttress Augmenter",
            "Luminous Ostentatious Prestige Augmenter",
            "Luminous Vitreous Stimulative Augmenter",
            "Lustrous Mass Despoliation Augmenter",
            "Lustrous Preposterous Fusillade Augmenter",
            "Glamorous Crepuscularized Potentiality Augmenter",
            "Glamorous Verdured Potentiality Augmenter",
            "Wondrous Postulant Approbation Augmenter",
            "Aberrant Ordnance Augmenter",
            "Aberrant Spearhead Augmenter",
            "Fallen Outrider Augmenter",
            "Operative's Accurized Conduits Augmenter",
            "Operative's Cosmic Symmetry Augmenter",
            "Operative's Guided Harmonizer Augmenter",
            "Operative's Stalking Tracker Augmenter"
        ],
        name_format="augmenter_setups_4_reduced.csv"
    )