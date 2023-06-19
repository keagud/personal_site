from django.http import HttpResponse
from django.shortcuts import render
from django.views import generic



# Create your views here.



class HomePageView(generic.TemplateView):
    template_name = "core/index.html"


#TODO make this more generic
class AboutPageView(generic.TemplateView):
    template_name = "core/about.html"

    
    
