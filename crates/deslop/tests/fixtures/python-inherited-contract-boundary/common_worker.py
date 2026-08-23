class CommonWorker:
    def __init__(self, clock):
        self.clock = clock

    def stamp(self):
        return self.clock.now()
