import requests
import io
from bs4 import BeautifulSoup as BS

import pandas as pd


def parse_bonuses(row):
    bonii = row['Bonuses'].split(", ")
    tmp = [i.rsplit(" ", 1) for i in bonii]

    bonus = [i[0] for i in tmp]
    mag = [i[1] for i in tmp]

    # add back the name and tech level
    idx = ['Name', 'Tech'] + bonus
    vals = [row['Name'], row['Tech']] + mag

    return pd.Series(vals, index=idx)


if __name__ == "__main__":
    r = requests.get("https://www.starsonata.com/wiki/index.php/Augmenters")

    parsed = BS(r.content, 'html.parser')

    # print(parsed.prettify)
    tables = parsed.find_all("div", "floating-table-label")

    dfs = [
        pd.read_html(io.StringIO(str(i)))[0] for i in tables
    ]

    df = pd.concat(dfs, ignore_index=True)

    # df = pd.read_html(io.StringIO(str(tables[-1])))[0]

    res = df.apply(parse_bonuses, axis=1)
    
    # move name and tech to front
    for c in ['Tech', 'Name']:
        col = res.pop(c)
        res.insert(0, col.name, col)
    
    # convert string percent to float
    for c in res.columns[2:]:
        res.loc[:, c] = res[c].str.rstrip("%").astype("float") / 100

    # save results
    res.to_csv("parsed_augmenters.csv", index=False)

