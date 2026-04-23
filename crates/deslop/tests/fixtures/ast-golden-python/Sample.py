from typing import Self


class Thing:
    @staticmethod
    async def probe(count: int) -> float:
        # dropped comment
        marker: str = f"{count}"
        flag: bool = True
        width: float = 1.5
        if (total := count + 1) > 0:
            pass
        match count:
            case 0:
                return 0.0
            case _:
                return float(count)
        return 0.0
