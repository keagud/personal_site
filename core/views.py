from django.shortcuts import render
from django.http import HttpResponse

from django.views import generic

# Create your views here.


class HomePageView(generic.TemplateView):
    template_name = "core/index.html"



