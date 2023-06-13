from pathlib import Path

from django.http import FileResponse, Http404, HttpResponse
from django.shortcuts import render

from personal_site.settings import STATIC_ROOT

import json




from django.views.generic import TemplateView

class ResumeView(TemplateView):
    template_name = "resume/resume.html"

    def get_context_data(self, **kwargs):
        print("foo")
        context = super().get_context_data(**kwargs)
        with open(STATIC_ROOT.joinpath('resume.json')) as context_file:
            context_data = json.load(context_file)

        context.update(context_data)
        return context



        






