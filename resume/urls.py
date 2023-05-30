from django.urls import path

from . import views

app_name = "polls"

urlpatterns = [
    path("", views.pdf_view, name="resume"),
]
