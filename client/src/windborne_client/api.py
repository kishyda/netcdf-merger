from __future__ import annotations

import dataclasses
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

from netCDF4 import Dataset

from .samples import create_netcdf_bytes_for_endpoint


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

    def read_response(self, name: str) -> ApiResponse:
        query = urlencode({"name": name})
        return self._request("GET", f"/read?{query}")

    def read(self, name: str) -> Dataset:
        return self._dataset_from_response(self.read_response(name))

    def upload(
        self,
        endpoint: str,
        name: str,
        dataset_name: str,
        netcdf_bytes: bytes | None = None,
    ) -> ApiResponse:
        normalized_endpoint = endpoint.strip().lstrip("/")
        query = urlencode({"name": name})
        if netcdf_bytes is None:
            request_body = create_netcdf_bytes_for_endpoint(
                normalized_endpoint,
                dataset_name,
            )
        else:
            request_body = netcdf_bytes
        return self._request(
            "POST",
            f"/{normalized_endpoint}?{query}",
            data=request_body,
            headers={"Content-Type": "application/netcdf"},
        )

    def part_a(self, name: str, dataset_name: str) -> ApiResponse:
        return self.upload("part_a", name=name, dataset_name=dataset_name)

    def part_b(self, name: str, dataset_name: str) -> ApiResponse:
        return self.upload("part_b", name=name, dataset_name=dataset_name)

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
