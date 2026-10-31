'''
tetron-wm main spec (packaging requirements only, WMDEB-*)
'''

from libspec import Spec
from . import packaging, constraints

class MainSpec(Spec):
    def modules(self):
        return [packaging, constraints]
