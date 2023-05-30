from django.apps import AppConfig
import subprocess
from pathlib import Path
from shutil import copy


class ResumeConfig(AppConfig):
    verbose_name = "My Resume"
    name = "resume"

    def ready(self):
        resume_build_path = Path(__file__).parent.joinpath("resume")
        build_script_path = resume_build_path.joinpath("build.py").as_posix()
        resume_pdf_path = resume_build_path.joinpath("resume.pdf").as_posix()
        build_step = subprocess.run(["python3", build_script_path])
        build_step.check_returncode()

        copy(resume_pdf_path, Path(__file__).parent)
