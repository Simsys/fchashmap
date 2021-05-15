import matplotlib.pyplot as plt

def plot_table(name, title, data):



    rel = []
    length = len(data[0])
    for i in range(512):
        rel.append(100.0*float(i)/float(length))

    insert_data = [float(element)/72.0 for element in data[0]]
    get_data = [float(element)/72.0 for element in data[1]]
    remove_data = [float(element)/72.0 for element in data[2]]

    fig, axs = plt.subplots(1, 3, figsize=(12.8, 4.2), dpi=100)
    axs[0].plot(rel, insert_data, color='C0')
    axs[0].set_title("map.insert(key, value)")
    axs[0].set_ylabel("Microseconds")
    axs[0].set_xlabel("Filling Level")

    axs[1].plot(rel, get_data, color='C3')
    axs[1].set_title("map.get(key)")
    axs[1].set_xlabel("Filling Level")

    axs[2].plot(rel, remove_data, color='C2')
    axs[2].set_title("map.remove(key)")
    axs[2].set_xlabel("Filling Level")

    fig.suptitle(title)

    plt.savefig(name + ".png")

import fchashmap
plot_table("fchashmap", "Performance FcHashMap 512 cm4 @ 72 MHz 'Release'", fchashmap.data)
import fncindexmap
plot_table("fnvindexmap", "Performance FnvIndexMap 512 cm4 @ 72 MHz 'Release'", fncindexmap.data)
