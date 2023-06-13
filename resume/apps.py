import subprocess
from pathlib import Path
from shutil import copy

from django.apps import AppConfig



class ResumeConfig(AppConfig):
    default_auto_field = "django.db.models.BigAutoField"
    name = "resume"
