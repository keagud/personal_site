"""
mdconvert
"""

import markdown
from pathlib import Path

from bs4 import BeautifulSoup


def convert(
    filepath: Path | str = "./test.md",
    output_file: Path | str | None = None,
    pretty: bool = True,
):
    filepath = Path(filepath)

    if output_file is None:
        output_filename = f"output_{filepath.stem}.html"
        output_file = filepath.parent.joinpath(output_filename)

    output_file = Path(output_file)

    with open(filepath, "r") as infile:
        file_contents = infile.read()

    extension_configs = {}

    processed_contents = markdown.markdown(
        file_contents,
        output_format="html",
        extensions=["extra"],
        extension_configs=extension_configs,
    )

    if pretty:
        soup = BeautifulSoup(processed_contents, "html.parser")
        processed_contents = soup.prettify()

    with open(output_file, "w") as outfile:
        outfile.write(processed_contents)


def main():
    convert()


if __name__ == "__main__":
    main()
