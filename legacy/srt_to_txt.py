import argparse
import os
import re
import sys

def clean_srt_content(content):
    """
    Removes SRT indices and timestamps from the content.
    """
    lines = content.splitlines()
    cleaned_lines = []
    
    # Regex for SRT timestamp: 00:00:00,000 --> 00:00:00,000
    timestamp_pattern = re.compile(r'\d{2}:\d{2}:\d{2},\d{3} --> \d{2}:\d{2}:\d{2},\d{3}')
    
    # Regex for integer index (only if it's a line by itself)
    index_pattern = re.compile(r'^\d+$')

    for line in lines:
        line = line.strip()
        
        # Skip empty lines
        if not line:
            continue
            
        # Skip timestamp lines
        if timestamp_pattern.match(line):
            continue
            
        # Skip index lines
        if index_pattern.match(line):
            continue
            
        cleaned_lines.append(line)
        
    return "\n".join(cleaned_lines)

def convert_file(file_path):
    """
    Converts a single SRT file to TXT.
    """
    if not file_path.lower().endswith('.srt'):
        print(f"Skipping non-SRT file: {file_path}")
        return

    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
            
        cleaned_text = clean_srt_content(content)
        
        output_path = os.path.splitext(file_path)[0] + '.txt'
        
        with open(output_path, 'w', encoding='utf-8') as f:
            f.write(cleaned_text)
            
        print(f"Converted: {file_path} -> {output_path}")
        
    except Exception as e:
        print(f"Error converting {file_path}: {e}")

def process_path(path):
    """
    Processes a file or directory.
    """
    if os.path.isfile(path):
        convert_file(path)
    elif os.path.isdir(path):
        for root, dirs, files in os.walk(path):
            for file in files:
                if file.lower().endswith('.srt'):
                    convert_file(os.path.join(root, file))
    else:
        print(f"Error: Path not found: {path}")

def main():
    parser = argparse.ArgumentParser(description="Convert SRT files to plain text (TXT), removing timestamps and indices.")
    parser.add_argument("path", help="Path to an SRT file or a directory containing SRT files")
    
    args = parser.parse_args()
    
    process_path(args.path)

if __name__ == "__main__":
    main()
