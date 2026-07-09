import re



if __name__ == "__main__":
    txt_file = "utils/wac.txt"
    with open(txt_file, "r") as file:
        data = file.readlines()
    
    print("let wac = [")
    
    for line in data:
        match = re.search(r"^(.*?)\s+bm\s+([^;]+);\s+id\s+\"([^\"]+)\";", line)
        if match:
            print(f"\t{match.groups()},")
    
    print("];")