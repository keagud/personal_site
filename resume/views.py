from pathlib import Path

from django.http import FileResponse, Http404, HttpResponse
from django.shortcuts import render

from personal_site.settings import STATICFILES_DIRS

import json


from django.views.generic import TemplateView


class ResumeView(TemplateView):
    template_name = "resume/resume_template.html"

    def get_context_data(self, **kwargs):
        context = super().get_context_data(**kwargs)
        with open(STATICFILES_DIRS[0].joinpath("resume/resume.json")) as context_file:
            context_data = json.load(context_file)

        context.update(context_data)
        return context
