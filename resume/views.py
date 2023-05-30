from django.shortcuts import render
from django.http import FileResponse, Http404, HttpResponse
from pathlib import Path


def pdf_view(request):
    pdf_path = Path(__file__).parent.joinpath("resume/resume.pdf")

    try:
        with open(pdf_path, "rb") as pdf:
            response = HttpResponse(pdf.read(), content_type="application/pdf")
            response["Content-Disposition"] = "inline;filename=resume.pdf"

            return response

    except FileNotFoundError:
        raise Http404()
