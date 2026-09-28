import csv

relative_offsets = []

with open('13.0.5.csv', mode='r', encoding='utf-8') as file:
    # Create the reader object
    csv_reader = csv.reader(file)
    
    # Optional: Skip the header row if your file has one
    header = next(csv_reader)
    #print(f"Header: {header}")
    
    # Iterate through the remaining rows
    for row in csv_reader:
        start = int(row[0], 16)
        end = int(row[1], 16)
        relative_offset = int(row[2], 16)
        relative_offsets.append([start, end, relative_offset])
        
import os

# Get the directory of the current script
script_dir = os.path.dirname(os.path.abspath(__file__))

import re
for root, dirs, files in os.walk(script_dir):
    for filename in files:
        if not filename.endswith(".rs"):
            continue
        file_path = os.path.join(root, filename)
        print(file_path)

        content = ""
        with open(file_path, 'r+', encoding="utf-8") as file:
            content = file.read()

                
        # Find all sequences of digits
        numbers = re.findall(r'0x[0-9a-fA-F]+', content)

        # Convert string numbers to integers (or floats if needed)
        for i, n in enumerate(numbers):
            int_n = int(n, 16)
            for relative_offset in relative_offsets:
                if int_n > relative_offset[0] and int_n < relative_offset[1]:
                    #print(f"{n}, {relative_offset[2]}")
                    if relative_offset[2] != 0:
                        replace = hex(int_n + relative_offset[2])
                        #print(replace)
                        content = content.replace(n, replace)
                            
            #print([int(n, 16) for n in numbers])


        with open(file_path, "w", encoding="utf-8") as file:
            file.write(content)
