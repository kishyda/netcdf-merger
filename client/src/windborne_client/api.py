from __future__ import annotations

import dataclasses
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

from netCDF4 import Dataset

from .samples import create_part_a_netcdf_bytes, create_part_b_netcdf_bytes


@dataclasses.dataclass(frozen=True)
class ApiResponse:
    status: int
    content_type: str | None
    body: bytes

    @property
    def text(self) -> str:
        return self.body.decode("utf-8")

    def is_netcdf(self) -> bool:
        return self.content_type == "application/netcdf"


class WindborneClient:
    def __init__(self, base_url: str = "http://127.0.0.1:8000") -> None:
        self.base_url = base_url.rstrip("/")

    def read(self, name: str) -> Dataset:
        query = urlencode({"name": name})
        response = self._request("GET", f"/read?{query}")
        return self._dataset_from_response(response)

    def part_a(self, name: str, dataset_name: str) -> ApiResponse:
        query = urlencode({"name": name})
        return self._request(
            "POST",
            f"/part_a?{query}",
            data=create_part_a_netcdf_bytes(dataset_name),
            headers={"Content-Type": "application/netcdf"},
        )

    def part_b(self, name: str, dataset_name: str) -> ApiResponse:
        query = urlencode({"name": name})
        return self._request(
            "POST",
            f"/part_b?{query}",
            data=create_part_b_netcdf_bytes(dataset_name),
            headers={"Content-Type": "application/netcdf"},
        )

    def _request(
        self,
        method: str,
        path: str,
        data: bytes | None = None,
        headers: dict[str, str] | None = None,
    ) -> ApiResponse:
        request = Request(
            url=f"{self.base_url}{path}",
            data=data,
            method=method,
            headers=headers or {},
        )
        try:
            with urlopen(request) as response:
                return ApiResponse(
                    status=response.status,
                    content_type=response.headers.get_content_type(),
                    body=response.read(),
                )
        except HTTPError as exc:
            body = exc.read().decode("utf-8", errors="replace")
            raise RuntimeError(f"HTTP {exc.code}: {body}") from exc
        except URLError as exc:
            raise RuntimeError(f"Request failed: {exc.reason}") from exc

    @staticmethod
    def _dataset_from_response(response: ApiResponse) -> Dataset:
        if not response.is_netcdf():
            raise RuntimeError(f"Expected application/netcdf, got {response.content_type}")
        return Dataset("inmemory.nc", mode="r", diskless=True, memory=response.body)
